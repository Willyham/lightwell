use crate::{
    AssetId, EditorService, EntryId, Error, ErrorKind, HistorySelection, Mutation, PreviewJob,
    PreviewSession, Transform, Zoom,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs::OpenOptions,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    thread::JoinHandle,
    time::Duration,
};

const MAX_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_CLIENTS: usize = 8;
const EVENT_CAPACITY: usize = 256;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiRequest {
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiFailure {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiResponse {
    pub id: String,
    pub sequence: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiFailure>,
}

impl ApiResponse {
    fn success(id: String, sequence: u64, result: impl Serialize) -> Self {
        match serde_json::to_value(result) {
            Ok(result) => Self {
                id,
                sequence,
                result: Some(result),
                error: None,
            },
            Err(error) => Self::failure(
                id,
                sequence,
                Error::new(ErrorKind::Internal, error.to_string()),
            ),
        }
    }
    fn failure(id: String, sequence: u64, error: Error) -> Self {
        Self {
            id,
            sequence,
            result: None,
            error: Some(ApiFailure {
                code: error.kind.code().into(),
                message: error.detail,
            }),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiEvent {
    pub sequence: u64,
    pub method: String,
    pub request_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventsResult {
    pub events: Vec<ApiEvent>,
    pub current_sequence: u64,
    pub gap: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientSession {
    pub preview: PreviewSession,
}

struct OwnerCall {
    request: ApiRequest,
    session: ClientSession,
    response: SyncSender<(ApiResponse, ClientSession)>,
}

enum OwnerMessage {
    Call(OwnerCall),
    Preview {
        asset_id: AssetId,
        entry_id: Option<EntryId>,
        response: SyncSender<Result<PreviewJob, Error>>,
    },
    Stop,
}

#[derive(Clone)]
pub struct OwnerHandle {
    sender: SyncSender<OwnerMessage>,
}

impl OwnerHandle {
    pub fn start(catalog: &Path) -> Result<(Self, JoinHandle<()>), Error> {
        let service = EditorService::open(catalog)?;
        let (sender, receiver) = sync_channel(64);
        let join = std::thread::spawn(move || owner_loop(service, receiver));
        Ok((Self { sender }, join))
    }

    pub fn call(
        &self,
        session: &mut ClientSession,
        request: ApiRequest,
    ) -> Result<ApiResponse, Error> {
        let (sender, receiver) = sync_channel(1);
        self.sender
            .send(OwnerMessage::Call(OwnerCall {
                request,
                session: session.clone(),
                response: sender,
            }))
            .map_err(|_| Error::new(ErrorKind::Protocol, "catalog owner is unavailable"))?;
        let (response, updated) = receiver.recv().map_err(|_| {
            Error::new(
                ErrorKind::Protocol,
                "catalog owner stopped before responding",
            )
        })?;
        *session = updated;
        Ok(response)
    }

    pub fn stop(&self) {
        let _ = self.sender.send(OwnerMessage::Stop);
    }

    pub fn preview_job(
        &self,
        asset_id: AssetId,
        entry_id: Option<EntryId>,
    ) -> Result<PreviewJob, Error> {
        let (response, receiver) = sync_channel(1);
        self.sender
            .send(OwnerMessage::Preview {
                asset_id,
                entry_id,
                response,
            })
            .map_err(|_| Error::new(ErrorKind::Protocol, "catalog owner is unavailable"))?;
        receiver
            .recv()
            .map_err(|_| Error::new(ErrorKind::Protocol, "catalog owner stopped before preview"))?
    }
}

fn owner_loop(mut service: EditorService, receiver: Receiver<OwnerMessage>) {
    let mut sequence = 0u64;
    let mut events = VecDeque::with_capacity(EVENT_CAPACITY);
    while let Ok(message) = receiver.recv() {
        match message {
            OwnerMessage::Stop => break,
            OwnerMessage::Preview {
                asset_id,
                entry_id,
                response,
            } => {
                let _ = response.send(service.preview_job(&asset_id, entry_id.as_ref()));
            }
            OwnerMessage::Call(call) => {
                let mut session = call.session;
                let id = call.request.id.clone();
                let response = if call.request.method == "events.since" {
                    event_response(&call.request, sequence, &events)
                } else {
                    let response = dispatch(&mut service, &mut session, &call.request, sequence);
                    if response.error.is_none()
                        && mutates(&call.request.method, response.result.as_ref())
                    {
                        sequence = sequence.saturating_add(1);
                        if events.len() == EVENT_CAPACITY {
                            events.pop_front();
                        }
                        events.push_back(ApiEvent {
                            sequence,
                            method: call.request.method.clone(),
                            request_id: id,
                        });
                        ApiResponse {
                            sequence,
                            ..response
                        }
                    } else {
                        response
                    }
                };
                let _ = call.response.send((response, session));
            }
        }
    }
}

fn mutates(method: &str, result: Option<&Value>) -> bool {
    if method == "catalog.import" {
        return true;
    }
    if !matches!(
        method,
        "edit.set-pixel" | "edit.transform" | "history.undo" | "history.redo" | "history.restore"
    ) {
        return false;
    }
    result
        .and_then(|value| value.get("outcome"))
        .and_then(Value::as_str)
        != Some("no-op")
}

fn event_response(request: &ApiRequest, sequence: u64, events: &VecDeque<ApiEvent>) -> ApiResponse {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Params {
        after: u64,
    }
    match params::<Params>(&request.params) {
        Ok(params) => {
            let oldest = events
                .front()
                .map(|event| event.sequence)
                .unwrap_or(sequence.saturating_add(1));
            let gap = params.after.saturating_add(1) < oldest;
            let selected = events
                .iter()
                .filter(|event| event.sequence > params.after)
                .cloned()
                .collect();
            ApiResponse::success(
                request.id.clone(),
                sequence,
                EventsResult {
                    events: selected,
                    current_sequence: sequence,
                    gap,
                },
            )
        }
        Err(error) => ApiResponse::failure(request.id.clone(), sequence, error),
    }
}

fn dispatch(
    service: &mut EditorService,
    session: &mut ClientSession,
    request: &ApiRequest,
    sequence: u64,
) -> ApiResponse {
    match dispatch_result(service, session, request) {
        Ok(result) => ApiResponse::success(request.id.clone(), sequence, result),
        Err(error) => ApiResponse::failure(request.id.clone(), sequence, error),
    }
}

fn dispatch_result(
    service: &mut EditorService,
    session: &mut ClientSession,
    request: &ApiRequest,
) -> Result<Value, Error> {
    match request.method.as_str() {
        "schema.list" => Ok(schemas()),
        "catalog.import" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                path: PathBuf,
            }
            Ok(value(service.import(&params::<P>(&request.params)?.path)?)?)
        }
        "asset.state" => {
            let p = asset_params(&request.params)?;
            Ok(value(service.state(&p.asset_id)?)?)
        }
        "history.list" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                asset_id: AssetId,
                before_sequence: Option<u64>,
                limit: Option<usize>,
            }
            let p = params::<P>(&request.params)?;
            Ok(value(service.history(
                &p.asset_id,
                p.before_sequence,
                p.limit.unwrap_or(50),
            )?)?)
        }
        "history.inspect" => {
            let p = entry_params(&request.params)?;
            Ok(value(service.entry(&p.asset_id, &p.entry_id)?)?)
        }
        "edit.set-pixel" => {
            require_current(session)?;
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                asset_id: AssetId,
                mutation: Mutation,
                x: u32,
                y: u32,
                rgb: [u8; 3],
            }
            let p = params::<P>(&request.params)?;
            Ok(value(service.apply_pixel(
                &p.asset_id,
                p.mutation,
                p.x,
                p.y,
                p.rgb,
            )?)?)
        }
        "edit.transform" => {
            require_current(session)?;
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                asset_id: AssetId,
                mutation: Mutation,
                transform: Transform,
            }
            let p = params::<P>(&request.params)?;
            Ok(value(service.apply_transform(
                &p.asset_id,
                p.mutation,
                p.transform,
            )?)?)
        }
        "history.undo" | "history.redo" => {
            require_current(session)?;
            let p = mutation_params(&request.params)?;
            let result = if request.method == "history.undo" {
                service.undo(&p.asset_id, p.mutation)?
            } else {
                service.redo(&p.asset_id, p.mutation)?
            };
            Ok(value(result)?)
        }
        "history.restore" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                asset_id: AssetId,
                mutation: Mutation,
                entry_id: EntryId,
            }
            let p = params::<P>(&request.params)?;
            let result = service.restore(&p.asset_id, p.mutation, &p.entry_id)?;
            session.preview.return_current();
            Ok(value(result)?)
        }
        "preview.select" => {
            let p = entry_params(&request.params)?;
            service.entry(&p.asset_id, &p.entry_id)?;
            let generation = session.preview.select(HistorySelection::Entry(p.entry_id));
            Ok(json!({"generation":generation,"session":session}))
        }
        "preview.return-current" => {
            Ok(json!({"generation":session.preview.return_current(),"session":session}))
        }
        "view.set" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                zoom: Option<Zoom>,
                pan_x: Option<f32>,
                pan_y: Option<f32>,
            }
            let p = params::<P>(&request.params)?;
            if let Some(zoom) = p.zoom {
                session.preview.view.set_zoom(zoom)?;
            }
            if p.pan_x.is_some() || p.pan_y.is_some() {
                session.preview.view.pan_to(
                    p.pan_x.unwrap_or(session.preview.view.pan_x),
                    p.pan_y.unwrap_or(session.preview.view.pan_y),
                )?;
            }
            Ok(value(session)?)
        }
        "session.state" => Ok(value(session)?),
        "render.sample" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                asset_id: AssetId,
                x: u32,
                y: u32,
            }
            let p = params::<P>(&request.params)?;
            let entry = match &session.preview.selection {
                HistorySelection::Current => service.state(&p.asset_id)?.current_entry,
                HistorySelection::Entry(id) => service.entry(&p.asset_id, id)?,
            };
            let raster = service.render_entry(&p.asset_id, &entry.id)?;
            let pixel = raster.pixel(p.x, p.y).ok_or_else(|| {
                Error::new(
                    ErrorKind::Validation,
                    "sample coordinate is outside rendered image",
                )
            })?;
            Ok(
                json!({"entry_id":entry.id,"snapshot_id":raster.snapshot_id,"source_fingerprint":raster.source_fingerprint,"width":raster.width,"height":raster.height,"x":p.x,"y":p.y,"rgba":pixel,"source_detail_ready":true}),
            )
        }
        _ => Err(Error::new(
            ErrorKind::Protocol,
            format!("unknown method {}", request.method),
        )),
    }
}

fn require_current(session: &ClientSession) -> Result<(), Error> {
    if session.preview.can_edit() {
        Ok(())
    } else {
        Err(Error::new(
            ErrorKind::Conflict,
            "return to current or restore the selected history entry before editing",
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetParams {
    asset_id: AssetId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryParams {
    asset_id: AssetId,
    entry_id: EntryId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MutationParams {
    asset_id: AssetId,
    mutation: Mutation,
}
fn asset_params(value: &Value) -> Result<AssetParams, Error> {
    params(value)
}
fn entry_params(value: &Value) -> Result<EntryParams, Error> {
    params(value)
}
fn mutation_params(value: &Value) -> Result<MutationParams, Error> {
    params(value)
}

fn params<T: DeserializeOwned>(value: &Value) -> Result<T, Error> {
    serde_json::from_value(value.clone())
        .map_err(|error| Error::new(ErrorKind::Validation, error.to_string()))
}
fn value(value: impl Serialize) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|error| Error::new(ErrorKind::Internal, error.to_string()))
}

pub fn schemas() -> Value {
    json!({
        "protocol":"lightwell-jsonl-1",
        "coordinate_space":"Each edit uses integer coordinates in its input image stage after EXIF orientation.",
        "methods":{
            "schema.list":{"mutates":false,"params":{}},
            "catalog.import":{"mutates":true,"required":["path"]},
            "asset.state":{"mutates":false,"required":["asset_id"]},
            "history.list":{"mutates":false,"required":["asset_id"],"optional":{"before_sequence":"u64","limit":"1..100"}},
            "history.inspect":{"mutates":false,"required":["asset_id","entry_id"]},
            "edit.set-pixel":{"mutates":true,"required":["asset_id","mutation","x","y","rgb"],"rgb":"three u8 sRGB channels"},
            "edit.transform":{"mutates":true,"required":["asset_id","mutation","transform"],"transform":["rotate-left","rotate-right","mirror-horizontal","flip-vertical"]},
            "history.undo":{"mutates":true,"required":["asset_id","mutation"]},
            "history.redo":{"mutates":true,"required":["asset_id","mutation"]},
            "history.restore":{"mutates":true,"required":["asset_id","mutation","entry_id"]},
            "preview.select":{"mutates":false,"required":["asset_id","entry_id"]},
            "preview.return-current":{"mutates":false,"params":{}},
            "view.set":{"mutates":false,"optional":{"zoom":"fit or percent 10..1600","pan_x":"finite","pan_y":"finite"}},
            "session.state":{"mutates":false,"params":{}},
            "render.sample":{"mutates":false,"required":["asset_id","x","y"]},
            "events.since":{"mutates":false,"required":["after"],"notes":"gap=true requires an asset.state refresh"}
        },
        "mutation":{"required":["expected_revision","request_id","actor"]}
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalSessionInfo {
    pub protocol: String,
    pub address: SocketAddr,
    pub token: String,
}

pub struct LocalServer {
    info: LocalSessionInfo,
    session_file: PathBuf,
    stopping: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl LocalServer {
    pub fn start(owner: OwnerHandle, session_file: &Path) -> Result<Self, Error> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| Error::new(ErrorKind::Protocol, error.to_string()))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| Error::new(ErrorKind::Protocol, error.to_string()))?;
        let info = LocalSessionInfo {
            protocol: "lightwell-jsonl-1".into(),
            address: listener
                .local_addr()
                .map_err(|error| Error::new(ErrorKind::Protocol, error.to_string()))?,
            token: uuid::Uuid::new_v4().simple().to_string(),
        };
        write_session_file(session_file, &info)?;
        let stopping = Arc::new(AtomicBool::new(false));
        let stop = stopping.clone();
        let token = info.token.clone();
        let clients = Arc::new(AtomicUsize::new(0));
        let join = std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if clients.fetch_add(1, Ordering::AcqRel) >= MAX_CLIENTS {
                            clients.fetch_sub(1, Ordering::AcqRel);
                            continue;
                        }
                        let owner = owner.clone();
                        let token = token.clone();
                        let clients = clients.clone();
                        std::thread::spawn(move || {
                            let _ = serve_stream(stream, &owner, Some(&token));
                            clients.fetch_sub(1, Ordering::AcqRel);
                        });
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            info,
            session_file: session_file.into(),
            stopping,
            join: Some(join),
        })
    }
    pub fn info(&self) -> &LocalSessionInfo {
        &self.info
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
        let _ = std::fs::remove_file(&self.session_file);
    }
}

fn write_session_file(path: &Path, info: &LocalSessionInfo) -> Result<(), Error> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|error| Error::new(ErrorKind::Protocol, error.to_string()))?;
    }
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|error| {
        Error::new(
            ErrorKind::Protocol,
            format!("cannot create live session file: {}", error.kind()),
        )
    })?;
    serde_json::to_writer(&mut file, info)
        .map_err(|error| Error::new(ErrorKind::Protocol, error.to_string()))?;
    file.flush()
        .map_err(|error| Error::new(ErrorKind::Protocol, error.to_string()))
}

pub fn serve_json_lines(
    reader: impl Read,
    writer: impl Write,
    owner: &OwnerHandle,
) -> Result<(), Error> {
    serve(reader, writer, owner, None)
}
fn serve_stream(stream: TcpStream, owner: &OwnerHandle, token: Option<&str>) -> Result<(), Error> {
    let writer = stream
        .try_clone()
        .map_err(|error| Error::new(ErrorKind::Protocol, error.to_string()))?;
    serve(stream, writer, owner, token)
}
fn serve(
    reader: impl Read,
    mut writer: impl Write,
    owner: &OwnerHandle,
    token: Option<&str>,
) -> Result<(), Error> {
    let mut reader = BufReader::new(reader);
    let mut session = ClientSession::default();
    loop {
        let mut line = String::new();
        let bytes = reader
            .by_ref()
            .take((MAX_REQUEST_BYTES + 1) as u64)
            .read_line(&mut line)
            .map_err(|error| Error::new(ErrorKind::Protocol, error.to_string()))?;
        if bytes == 0 {
            return Ok(());
        }
        let response = if bytes > MAX_REQUEST_BYTES || !line.ends_with('\n') {
            ApiResponse::failure(
                "".into(),
                0,
                Error::new(ErrorKind::Protocol, "request exceeds JSON line limit"),
            )
        } else {
            match serde_json::from_str::<ApiRequest>(&line) {
                Ok(request) if token.is_some() && request.token.as_deref() != token => {
                    ApiResponse::failure(
                        request.id,
                        0,
                        Error::new(ErrorKind::Protocol, "invalid live-session token"),
                    )
                }
                Ok(request) => owner.call(&mut session, request)?,
                Err(error) => ApiResponse::failure(
                    "".into(),
                    0,
                    Error::new(ErrorKind::Protocol, error.to_string()),
                ),
            }
        };
        serde_json::to_writer(&mut writer, &response)
            .map_err(|error| Error::new(ErrorKind::Protocol, error.to_string()))?;
        writer
            .write_all(b"\n")
            .and_then(|_| writer.flush())
            .map_err(|error| Error::new(ErrorKind::Protocol, error.to_string()))?;
        if bytes > MAX_REQUEST_BYTES {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Cursor,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(1);
    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "lightwell-api-{}-{}-{name}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }
    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/s0/orientation-1.jpg")
    }
    fn request(id: &str, method: &str, params: Value) -> String {
        format!("{}\n", json!({"id":id,"method":method,"params":params}))
    }

    #[test]
    fn independent_json_client_drives_persistent_history_and_preview() {
        let catalog = temp("catalog.sqlite");
        let (owner, join) = OwnerHandle::start(&catalog).unwrap();
        let input = request("import", "catalog.import", json!({"path":fixture()}));
        let mut output = Vec::new();
        serve_json_lines(Cursor::new(input), &mut output, &owner).unwrap();
        let imported: ApiResponse = serde_json::from_slice(&output).unwrap();
        let asset = imported.result.unwrap()["asset"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let input = [
            request("schema", "schema.list", json!({})),
            request("pixel", "edit.set-pixel", json!({"asset_id":asset,"mutation":{"expected_revision":0,"request_id":"p1","actor":"api-test"},"x":0,"y":0,"rgb":[1,2,3]})),
            request("sample", "render.sample", json!({"asset_id":asset,"x":0,"y":0})),
            request("undo", "history.undo", json!({"asset_id":asset,"mutation":{"expected_revision":1,"request_id":"u1","actor":"api-test"}})),
        ]
        .concat();
        output.clear();
        serve_json_lines(Cursor::new(input), &mut output, &owner).unwrap();
        let responses: Vec<ApiResponse> = output
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect();
        assert_eq!(responses.len(), 4);
        assert!(responses.iter().all(|response| response.error.is_none()));
        assert_eq!(
            responses[2].result.as_ref().unwrap()["rgba"],
            json!([1, 2, 3, 255])
        );
        owner.stop();
        join.join().unwrap();
        std::fs::remove_file(catalog).unwrap();
    }

    #[test]
    fn local_server_requires_token_and_removes_session_file() {
        let catalog = temp("live.sqlite");
        let session_file = temp("session.json");
        let (owner, join) = OwnerHandle::start(&catalog).unwrap();
        {
            let server = match LocalServer::start(owner.clone(), &session_file) {
                Ok(server) => server,
                Err(error) if error.detail.contains("Operation not permitted") => {
                    owner.stop();
                    join.join().unwrap();
                    std::fs::remove_file(catalog).unwrap();
                    return;
                }
                Err(error) => panic!("cannot start local server: {error}"),
            };
            let mut stream = TcpStream::connect(server.info().address).unwrap();
            stream
                .write_all(request("bad", "schema.list", json!({})).as_bytes())
                .unwrap();
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line).unwrap();
            let response: ApiResponse = serde_json::from_str(&line).unwrap();
            assert_eq!(response.error.unwrap().code, "protocol");
        }
        assert!(!session_file.exists());
        owner.stop();
        join.join().unwrap();
        std::fs::remove_file(catalog).unwrap();
    }

    #[test]
    fn historical_preview_stays_selected_during_another_clients_commit() {
        let catalog = temp("preview-live.sqlite");
        let (owner, join) = OwnerHandle::start(&catalog).unwrap();
        let mut viewer = ClientSession::default();
        let mut agent = ClientSession::default();
        let call = |owner: &OwnerHandle,
                    session: &mut ClientSession,
                    id: &str,
                    method: &str,
                    params: Value| {
            owner
                .call(
                    session,
                    ApiRequest {
                        id: id.into(),
                        method: method.into(),
                        params,
                        token: None,
                    },
                )
                .unwrap()
        };
        let imported = call(
            &owner,
            &mut viewer,
            "import",
            "catalog.import",
            json!({"path":fixture()}),
        );
        let result = imported.result.unwrap();
        let asset = result["asset"]["id"].clone();
        let original = result["current_entry"]["id"].clone();
        let baseline = call(
            &owner,
            &mut viewer,
            "baseline",
            "render.sample",
            json!({"asset_id":asset,"x":0,"y":0}),
        )
        .result
        .unwrap()["rgba"]
            .clone();
        call(
            &owner,
            &mut viewer,
            "a",
            "edit.set-pixel",
            json!({"asset_id":asset,"mutation":{"expected_revision":0,"request_id":"a","actor":"viewer"},"x":0,"y":0,"rgb":[1,2,3]}),
        );
        call(
            &owner,
            &mut viewer,
            "preview",
            "preview.select",
            json!({"asset_id":asset,"entry_id":original}),
        );
        call(
            &owner,
            &mut agent,
            "b",
            "edit.set-pixel",
            json!({"asset_id":asset,"mutation":{"expected_revision":1,"request_id":"b","actor":"agent"},"x":0,"y":0,"rgb":[9,8,7]}),
        );
        let preview = call(
            &owner,
            &mut viewer,
            "sample-old",
            "render.sample",
            json!({"asset_id":asset,"x":0,"y":0}),
        );
        assert_eq!(preview.result.unwrap()["rgba"], baseline);
        call(
            &owner,
            &mut viewer,
            "current",
            "preview.return-current",
            json!({}),
        );
        let current = call(
            &owner,
            &mut viewer,
            "sample-current",
            "render.sample",
            json!({"asset_id":asset,"x":0,"y":0}),
        );
        assert_eq!(current.result.unwrap()["rgba"], json!([9, 8, 7, 255]));
        owner.stop();
        join.join().unwrap();
        std::fs::remove_file(catalog).unwrap();
    }
}
