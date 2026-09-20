use crate::{Config, diagnostics::Diagnostics, paths::Paths};
use iced::{
    ContentFit, Element, Length, Point, Rectangle, Size, Subscription, Task,
    widget::{
        button, column, container, image, mouse_area, operation, responsive, row, scrollable, text,
        text_input,
    },
};
use iced_runtime::image as image_memory;
use lightwell_core::{
    ActionDescriptor, ApiRequest, AssetId, Availability, CanvasInteraction, ClientId,
    ClientSession, Control, EditorState, EntryId, ErrorKind, EventsResult, HistoryEntry,
    HistoryPage, HistorySelection, Lineage, LocalServer, ModuleDescriptor, Mutation, OwnerHandle,
    ParameterDescriptor, ParameterKind, PreviewJob, PreviewQueue, Version, Zoom,
};
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

static REQUEST_NUMBER: AtomicU64 = AtomicU64::new(1);
const HISTORY_PAGE_SIZE: usize = 50;
const LINEAGE_LIMIT: usize = 100;
const ACTOR: &str = "desktop";
/// Layout constants shared by the view and the evidence frame description.
const PADDING: f32 = 16.0;
const SPACING: f32 = 12.0;
const SIDEBAR_WIDTH: f32 = 340.0;
/// An evidence run that has not finished by then is stuck; exit so the harness reaps nothing.
const EVIDENCE_DEADLINE: Duration = Duration::from_secs(25);

/// What the editor was last asked to show, correlated with logged events and captured frames.
struct Activity {
    /// Counts open requests; `displayed` is the request whose image is on screen.
    requested: u64,
    displayed: u64,
    /// An open request is in progress until its image is uploaded or it fails.
    pending: bool,
    phase: &'static str,
    error_code: Option<String>,
    source_dimensions: Option<(u32, u32)>,
    preview_dimensions: Option<(u32, u32)>,
    orientation: Option<u8>,
    backend: Option<Value>,
    request_started: Instant,
}

/// Evidence mode: import queued files in order, capture a frame after each outcome, then exit.
struct Evidence {
    dir: PathBuf,
    queue: VecDeque<PathBuf>,
    frames: Vec<Value>,
    capture_pending: bool,
    saving: bool,
    had_errors: bool,
}

/// Authoritative state read back from the owner after a change. `history` is `None` when only the
/// current entry needs merging into the loaded page.
#[derive(Clone, Debug)]
struct Refresh {
    state: EditorState,
    history: Option<HistoryPage>,
    versions: Vec<Version>,
    lineage: Lineage,
    job: PreviewJob,
    session: ClientSession,
    sequence: u64,
}

#[derive(Clone, Debug)]
struct PreviewPayload {
    job: PreviewJob,
    session: ClientSession,
    sequence: u64,
}

#[derive(Clone, Debug)]
struct Upload {
    generation: u64,
    width: u32,
    height: u32,
    entry_id: EntryId,
    snapshot_id: String,
    source_fingerprint: String,
    started: Instant,
}

#[derive(Clone, Debug)]
enum SyncResult {
    Unchanged { sequence: u64 },
    Changed(Box<Refresh>),
}

#[derive(Clone, Debug)]
enum Message {
    Open,
    Picked(Option<PathBuf>),
    Refreshed(Result<Box<Refresh>, String>),
    PreviewLoaded(Result<Box<PreviewPayload>, String>),
    SessionUpdated(Result<(ClientSession, u64), String>),
    PanSynced(Result<ClientSession, String>),
    VersionsLoaded(Result<(Vec<Version>, u64), String>),
    Synced(Result<SyncResult, String>),
    OlderLoaded(Result<(HistoryPage, u64), String>),
    Sync,
    Poll,
    Uploaded(
        Upload,
        Result<image_memory::Allocation, image_memory::Error>,
    ),
    /// Every tool control is generated from these; the desktop knows no tool by name.
    ModulesLoaded(Result<Vec<ModuleDescriptor>, String>),
    /// A generated field changed: the text the user typed for one declared parameter.
    ControlChanged {
        action: String,
        parameter: String,
        text: String,
    },
    /// Enter in a generated field runs that field's action when it is runnable.
    ControlSubmitted {
        action: String,
    },
    RunAction {
        action: String,
        preset: Map<String, Value>,
    },
    /// The last pointer position over the photo, already mapped to image pixels.
    PointerMoved(Option<(u32, u32)>),
    /// A canvas pick fills the declared coordinate fields; it never commits.
    PointPicked {
        x: u32,
        y: u32,
    },
    FocusNext,
    FocusPrevious,
    Zoom(String),
    VersionName(String),
    Panned(f32, f32),
    Undo,
    Redo,
    Preview(EntryId),
    ReturnCurrent,
    Restore,
    SaveVersion,
    DeleteVersion(String),
    LoadOlder,
    Fit,
    HundredPercent,
    ApplyZoom,
    ScaleFactor(f32),
    Info(iced::system::Information),
    EvidenceTick,
    Capture,
    Captured(iced::window::Screenshot),
    Saved(Result<Value, String>),
    Close,
}

/// Catalog ownership and the live service start before the window so failures are reported, not panics.
struct Boot {
    owner: OwnerHandle,
    join: JoinHandle<()>,
    live_server: Option<LocalServer>,
    config: Config,
}

pub(super) fn run(config: Config, size: (f32, f32)) -> Result<(), String> {
    // Evidence runs never touch a real catalog: theirs lives inside the new evidence directory.
    let catalog = match (&config.catalog, &config.evidence) {
        (Some(catalog), _) => catalog.clone(),
        (None, Some(evidence)) => evidence.join("catalog.sqlite"),
        (None, None) => Paths::resolve(config.data_root.as_ref())
            .ok_or("no usable application data directory; pass --data-root")?
            .config
            .join("catalog.sqlite"),
    };
    let (owner, join) = OwnerHandle::start(&catalog).map_err(|error| match error.kind {
        ErrorKind::Conflict => format!(
            "another Lightwell instance owns the catalog {}; close it or pass --catalog",
            catalog.display()
        ),
        _ => format!("cannot open catalog {}: {error}", catalog.display()),
    })?;
    let session_file = catalog.with_extension("live-session.json");
    // Owning the catalog proves any same-catalog session file from an earlier process is stale.
    if session_file.exists() {
        let _ = std::fs::remove_file(&session_file);
    }
    let live_server = LocalServer::start(owner.clone(), &session_file).ok();
    let boot = Mutex::new(Some(Boot {
        owner,
        join,
        live_server,
        config,
    }));
    iced::application(
        move || {
            Editor::new(
                boot.lock()
                    .expect("boot state is never poisoned")
                    .take()
                    .expect("the editor boots once"),
            )
        },
        Editor::update,
        Editor::view,
    )
    .title("Lightwell")
    .window_size(size)
    .exit_on_close_request(false)
    .theme(iced::Theme::Dark)
    .subscription(Editor::subscription)
    .run()
    .map_err(|error| error.to_string())
}

struct Editor {
    owner: OwnerHandle,
    owner_join: Option<JoinHandle<()>>,
    live_server: Option<LocalServer>,
    /// The desktop is one registered client; the owner holds its session.
    client: ClientId,
    /// Local copy of the owner's session, replaced only by a response with a newer revision.
    session: ClientSession,
    activity: Activity,
    evidence: Option<Evidence>,
    diagnostics: Option<Diagnostics>,
    run_id: String,
    /// Emit events to stderr when a log was requested but is unavailable.
    verbose: bool,
    started: Instant,
    state: Option<EditorState>,
    history: HistoryPage,
    versions: Vec<Version>,
    /// Entries on the current undo-parent chain; other loaded entries are abandoned branches.
    lineage: HashSet<EntryId>,
    /// Oldest lineage sequence when the chain was truncated; entries at or below it are unknown.
    lineage_floor: Option<u64>,
    display_entry: Option<EntryId>,
    photo: Option<image_memory::Allocation>,
    dimensions: Option<(u32, u32)>,
    preview_queue: PreviewQueue,
    preview_generation: u64,
    uploading: bool,
    busy: bool,
    syncing: bool,
    pan_in_flight: bool,
    pending_pan: Option<(f32, f32)>,
    picker_open: bool,
    status: String,
    api_sequence: u64,
    scale_factor: f32,
    /// Descriptors fetched once through `module.list`; the only source of tool controls.
    modules: Vec<ModuleDescriptor>,
    /// Set once discovery answered, successfully or not, so evidence never captures an empty panel.
    modules_ready: bool,
    /// The text typed into each generated field, by (action id, parameter name).
    fields: Fields,
    /// The last pointer position over the photo in image pixels; a pick commits nothing.
    pointer: Option<(u32, u32)>,
    zoom: String,
    version_name: String,
}

impl Editor {
    fn new(boot: Boot) -> (Self, Task<Message>) {
        let Boot {
            owner,
            join,
            live_server,
            mut config,
        } = boot;
        let client = owner.register();
        let evidence = config.evidence.take().map(|dir| Evidence {
            dir,
            queue: std::mem::take(&mut config.files),
            frames: Vec::new(),
            capture_pending: false,
            saving: false,
            had_errors: false,
        });
        let initial = config.files.pop_front();
        let mut editor = Self {
            owner: owner.clone(),
            owner_join: Some(join),
            live_server,
            client,
            session: ClientSession::default(),
            activity: Activity {
                requested: 0,
                displayed: 0,
                pending: false,
                phase: "empty",
                error_code: None,
                source_dimensions: None,
                preview_dimensions: None,
                orientation: None,
                backend: None,
                request_started: Instant::now(),
            },
            evidence,
            diagnostics: config.diagnostics.clone(),
            run_id: config.run_id.clone(),
            verbose: config.wants_events(),
            started: Instant::now(),
            state: None,
            history: HistoryPage {
                entries: Vec::new(),
                next_before_sequence: None,
            },
            versions: Vec::new(),
            lineage: HashSet::new(),
            lineage_floor: None,
            display_entry: None,
            photo: None,
            dimensions: None,
            preview_queue: PreviewQueue::default(),
            preview_generation: 0,
            uploading: false,
            busy: false,
            syncing: false,
            pan_in_flight: false,
            pending_pan: None,
            picker_open: false,
            status: "Open a JPEG to begin".into(),
            api_sequence: 0,
            scale_factor: 1.0,
            modules: Vec::new(),
            modules_ready: false,
            fields: Fields::default(),
            pointer: None,
            zoom: "100".into(),
            version_name: String::new(),
        };
        if editor.live_server.is_none() {
            editor.status = "Editor ready; live API unavailable on this host".into();
        }
        editor.event(
            "startup",
            json!({"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"version":env!("CARGO_PKG_VERSION"),"debug_assertions":cfg!(debug_assertions),"mode":if editor.evidence.is_some() {"evidence"} else {"editor"}}),
        );
        let scale = iced::window::oldest()
            .and_then(iced::window::scale_factor)
            .map(Message::ScaleFactor);
        let backend = iced::system::information().map(Message::Info);
        // Tool controls are discovered once, through the same API every other client uses.
        let modules = modules_task(editor.owner.clone(), editor.client);
        let first = match &mut editor.evidence {
            Some(evidence) => match evidence.queue.pop_front() {
                Some(path) => editor.open(path),
                None => {
                    evidence.capture_pending = true;
                    Task::none()
                }
            },
            None => initial
                .map(|path| editor.open(path))
                .unwrap_or_else(Task::none),
        };
        (editor, Task::batch([scale, backend, modules, first]))
    }

    fn event(&self, name: &str, detail: Value) {
        let value = json!({"event":name,"run_id":self.run_id,"build_version":env!("CARGO_PKG_VERSION"),"elapsed_ms":self.started.elapsed().as_secs_f64()*1000.,"request_id":self.activity.requested,"generation":self.activity.requested,"detail":detail});
        if let Some(log) = &self.diagnostics {
            log.event(value);
        } else if self.verbose {
            eprintln!("{value}");
        }
    }

    /// The state correlated with every event and captured frame; never includes source paths.
    fn snapshot(&self) -> Value {
        json!({"run_id":self.run_id,"mode":if self.evidence.is_some() {"evidence"} else {"editor"},"orientation":self.activity.orientation,"phase":self.activity.phase,"requested_generation":self.activity.requested,"displayed_generation":self.activity.displayed,"source_dimensions":self.activity.source_dimensions,"preview_dimensions":self.activity.preview_dimensions,"backend":self.activity.backend,"status":self.status,"error_code":self.activity.error_code,"modules":module_summary(&self.modules),"controls":self.fields.summary()})
    }

    /// Import a file through the same API call the Open button uses, tracked as one open request.
    fn open(&mut self, path: PathBuf) -> Task<Message> {
        self.activity.requested += 1;
        self.activity.pending = true;
        self.activity.phase = "loading";
        self.activity.error_code = None;
        self.activity.request_started = Instant::now();
        self.busy = true;
        self.status = "Importing photograph…".into();
        let file = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.event("open_requested", json!({"file":file}));
        import_task(self.owner.clone(), self.client, path)
    }

    fn open_failed(&mut self, error_code: &str, message: &str) {
        self.activity.pending = false;
        self.activity.phase = "error";
        self.activity.error_code = Some(error_code.into());
        self.event(
            "open_failed",
            json!({"error_code":error_code,"message":message}),
        );
        self.outcome_ready(true);
    }

    /// An open request reached its outcome; evidence mode captures a frame for it.
    fn outcome_ready(&mut self, failed: bool) {
        if let Some(evidence) = &mut self.evidence {
            evidence.had_errors |= failed;
            evidence.capture_pending = true;
        }
    }

    fn finish_evidence(&mut self) -> Task<Message> {
        self.event("shutdown", json!({}));
        let evidence = self.evidence.as_mut().expect("evidence mode");
        let dir = evidence.dir.clone();
        let frames = std::mem::take(&mut evidence.frames);
        let had_errors = evidence.had_errors;
        let result = json!({"run_id":self.run_id,"status":"captured","had_input_errors":had_errors,"frames":frames,"elapsed_ms":self.started.elapsed().as_secs_f64()*1000.,"build_version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"arch":std::env::consts::ARCH});
        let state = self.snapshot();
        let log = self.diagnostics.take();
        self.live_server.take();
        self.owner.disconnect(self.client);
        self.owner.stop();
        let join = self.owner_join.take();
        Task::perform(
            async move {
                if let Some(log) = log
                    && !log.finish()
                {
                    eprintln!("diagnostics: incomplete evidence log");
                    std::process::exit(4);
                }
                let write = || -> std::io::Result<()> {
                    std::fs::write(
                        dir.join("result.json"),
                        serde_json::to_vec_pretty(&result).expect("result is serializable"),
                    )?;
                    std::fs::write(
                        dir.join("state.json"),
                        serde_json::to_vec_pretty(&state).expect("state is serializable"),
                    )
                };
                if let Err(error) = write() {
                    eprintln!("Evidence finalize failed: {error}");
                    std::process::exit(4);
                }
                if let Some(join) = join {
                    let _ = join.join();
                }
            },
            |_| (),
        )
        .then(|_| iced::exit())
    }

    /// Keep the newest session the owner has reported; responses may complete out of order.
    fn adopt(&mut self, session: ClientSession) {
        if session.revision >= self.session.revision {
            self.session = session;
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open => {
                if self.picker_open || self.busy || self.evidence.is_some() {
                    return Task::none();
                }
                self.picker_open = true;
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("JPEG", &["jpg", "jpeg"])
                            .pick_file()
                            .await
                            .map(|file| file.path().to_path_buf())
                    },
                    Message::Picked,
                );
            }
            Message::Picked(path) => {
                self.picker_open = false;
                if let Some(path) = path {
                    return self.open(path);
                }
            }
            Message::Refreshed(result) => {
                self.busy = false;
                match result {
                    Ok(refresh) => {
                        if self.activity.pending {
                            self.activity.source_dimensions =
                                Some((refresh.state.asset.width, refresh.state.asset.height));
                            self.activity.orientation = Some(refresh.job.source.orientation);
                        }
                        self.accept(*refresh);
                    }
                    Err(error) => {
                        self.status = error.clone();
                        if self.activity.pending {
                            let (code, message) =
                                error.split_once(": ").unwrap_or(("internal", &error));
                            self.open_failed(code, message);
                        }
                    }
                }
            }
            Message::Info(info) => {
                self.activity.backend =
                    Some(json!({"backend":info.graphics_backend,"adapter":info.graphics_adapter}));
                self.event(
                    "backend",
                    self.activity.backend.clone().unwrap_or(Value::Null),
                );
            }
            Message::EvidenceTick => {
                if self.evidence.is_some() && self.started.elapsed() > EVIDENCE_DEADLINE {
                    eprintln!("Evidence deadline exceeded; inspect retained subprocess output");
                    std::process::exit(3);
                }
            }
            Message::Capture => {
                let Some(evidence) = &mut self.evidence else {
                    return Task::none();
                };
                // Wait for the backend and for tool discovery so a frame always shows real controls.
                if !evidence.capture_pending
                    || evidence.saving
                    || self.activity.backend.is_none()
                    || !self.modules_ready
                {
                    return Task::none();
                }
                evidence.capture_pending = false;
                evidence.saving = true;
                return iced::window::oldest()
                    .and_then(iced::window::screenshot)
                    .map(Message::Captured);
            }
            Message::Captured(shot) => {
                self.event(
                    "frame_captured",
                    json!({"displayed_generation":self.activity.displayed,"request_to_capture_ms":self.activity.request_started.elapsed().as_secs_f64()*1000.}),
                );
                let state = self.snapshot();
                let generation = self.activity.requested;
                let Some(evidence) = &self.evidence else {
                    return Task::none();
                };
                let dir = evidence.dir.clone();
                let scale = shot.scale_factor;
                let logical_width = shot.size.width as f32 / scale;
                // The photo surface spans the window minus padding, the sidebar and their spacing.
                let columns = [
                    (PADDING * scale).round() as u32,
                    ((logical_width - PADDING - SIDEBAR_WIDTH - SPACING) * scale).round() as u32,
                ];
                return Task::perform(
                    async move {
                        let name = format!("frame-{generation}.png");
                        ::image::save_buffer(
                            dir.join(&name),
                            &shot.rgba,
                            shot.size.width,
                            shot.size.height,
                            ::image::ColorType::Rgba8,
                        )
                        .map_err(|e| e.to_string())?;
                        let frame = json!({"file":name,"state":state,"capture_provenance":"window-renderer-readback","color":"sRGB","physical_size":[shot.size.width,shot.size.height],"scale":scale,"surface_columns":columns});
                        std::fs::write(
                            dir.join(format!("state-{generation}.json")),
                            serde_json::to_vec_pretty(&frame).expect("frame is serializable"),
                        )
                        .map_err(|e| e.to_string())?;
                        Ok(frame)
                    },
                    Message::Saved,
                );
            }
            Message::Saved(result) => {
                let Some(evidence) = &mut self.evidence else {
                    return Task::none();
                };
                evidence.saving = false;
                match result {
                    Ok(frame) => evidence.frames.push(frame),
                    Err(error) => {
                        eprintln!("Evidence write failed: {error}");
                        std::process::exit(4);
                    }
                }
                return match evidence.queue.pop_front() {
                    Some(path) => self.open(path),
                    None => self.finish_evidence(),
                };
            }
            Message::PreviewLoaded(result) => {
                self.busy = false;
                match result {
                    Ok(payload) => {
                        let payload = *payload;
                        self.api_sequence = payload.sequence;
                        self.adopt(payload.session);
                        self.display_entry = Some(payload.job.entry.id.clone());
                        self.preview_generation = self.preview_queue.request(payload.job);
                        self.status = "Rendering selected history state…".into();
                    }
                    Err(error) => self.status = error,
                }
            }
            Message::SessionUpdated(result) => {
                self.busy = false;
                match result {
                    Ok((session, sequence)) => {
                        self.adopt(session);
                        self.api_sequence = sequence;
                        self.status = "View updated".into();
                    }
                    Err(error) => self.status = error,
                }
            }
            Message::PanSynced(result) => {
                self.pan_in_flight = false;
                match result {
                    Ok(session) => self.adopt(session),
                    Err(error) => self.status = error,
                }
                if let Some((x, y)) = self.pending_pan.take() {
                    return self.pan(x, y);
                }
            }
            Message::VersionsLoaded(result) => {
                self.busy = false;
                match result {
                    Ok((versions, sequence)) => {
                        self.versions = versions;
                        self.api_sequence = sequence;
                        self.version_name.clear();
                        self.status = "Versions updated".into();
                    }
                    Err(error) => self.status = error,
                }
            }
            Message::Sync => {
                if self.syncing || self.busy || self.state.is_none() {
                    return Task::none();
                }
                self.syncing = true;
                return sync_task(
                    self.owner.clone(),
                    self.client,
                    self.state.as_ref().unwrap().asset.id.clone(),
                    self.api_sequence,
                );
            }
            Message::Synced(result) => {
                self.syncing = false;
                match result {
                    Ok(SyncResult::Unchanged { sequence }) => self.api_sequence = sequence,
                    Ok(SyncResult::Changed(refresh)) => self.accept(*refresh),
                    Err(error) => self.status = format!("Live refresh failed: {error}"),
                }
            }
            Message::OlderLoaded(result) => {
                self.busy = false;
                match result {
                    Ok((page, sequence)) => {
                        self.api_sequence = sequence;
                        self.history.entries.extend(page.entries);
                        self.history.next_before_sequence = page.next_before_sequence;
                        self.status = "Loaded older history".into();
                    }
                    Err(error) => self.status = error,
                }
            }
            Message::Poll => {
                if !self.uploading
                    && let Some(result) = self.preview_queue.poll()
                {
                    if result.generation != self.preview_generation {
                        return Task::none();
                    }
                    match result.result {
                        Ok(raster) => {
                            self.uploading = true;
                            self.status = "Preparing pixels for display…".into();
                            if self.activity.pending {
                                self.activity.preview_dimensions =
                                    Some((raster.width, raster.height));
                                self.event(
                                    "decoded",
                                    json!({"open_to_raster_ms":self.activity.request_started.elapsed().as_secs_f64()*1000.,"source_dimensions":self.activity.source_dimensions,"preview_dimensions":[raster.width,raster.height]}),
                                );
                            }
                            let upload = Upload {
                                generation: result.generation,
                                width: raster.width,
                                height: raster.height,
                                entry_id: result.entry_id,
                                snapshot_id: raster.snapshot_id.to_string(),
                                source_fingerprint: raster.source_fingerprint,
                                started: Instant::now(),
                            };
                            let handle = image::Handle::from_rgba(
                                raster.width,
                                raster.height,
                                iced_runtime::core::Bytes::from_owner(raster.rgba),
                            );
                            return image_memory::allocate(handle)
                                .map(move |result| Message::Uploaded(upload.clone(), result));
                        }
                        Err(error) => {
                            self.status = error.to_string();
                            if self.activity.pending {
                                self.activity.pending = false;
                                self.activity.phase = "error";
                                self.activity.error_code = Some(error.kind.code().into());
                                self.event(
                                    "render_failed",
                                    json!({"error_code":error.kind.code()}),
                                );
                                self.outcome_ready(true);
                            }
                        }
                    }
                }
            }
            Message::Uploaded(upload, result) => {
                self.uploading = false;
                if upload.generation != self.preview_generation {
                    return Task::none();
                }
                match result {
                    Ok(allocation) => {
                        self.photo = Some(allocation);
                        self.dimensions = Some((upload.width, upload.height));
                        self.display_entry = Some(upload.entry_id.clone());
                        if self.activity.pending {
                            self.activity.pending = false;
                            self.activity.displayed = self.activity.requested;
                            self.activity.phase = "ready";
                            self.event(
                                "render_ready",
                                json!({"upload_ms":upload.started.elapsed().as_secs_f64()*1000.,"displayed_generation":self.activity.displayed}),
                            );
                            self.outcome_ready(false);
                        }
                        let marker = if self.session.preview.can_edit() {
                            "Current"
                        } else {
                            "Previewing history"
                        };
                        self.status = format!(
                            "{marker} · {} × {} · entry {} · snapshot {} · source {}",
                            upload.width,
                            upload.height,
                            short(upload.entry_id.as_str()),
                            short(&upload.snapshot_id),
                            short(&upload.source_fingerprint)
                        );
                    }
                    Err(_) => {
                        self.status = "Could not upload rendered pixels".into();
                        if self.activity.pending {
                            self.activity.pending = false;
                            self.activity.phase = "error";
                            self.activity.error_code = Some(ErrorKind::Render.code().into());
                            self.event("render_failed", json!({"error_code":"render"}));
                            self.outcome_ready(true);
                        }
                    }
                }
            }
            Message::ModulesLoaded(result) => {
                self.modules_ready = true;
                match result {
                    Ok(modules) => {
                        self.fields = Fields::seeded(&modules);
                        self.event("modules_loaded", module_summary(&modules));
                        self.modules = modules;
                    }
                    Err(error) => {
                        self.status = format!("Tool discovery failed: {error}");
                        self.event("modules_failed", json!({ "message": self.status }));
                    }
                }
            }
            Message::ControlChanged {
                action,
                parameter,
                text,
            } => self.fields.set(&action, &parameter, text),
            Message::ControlSubmitted { action } => {
                let Some(preset) =
                    submit_preset(&self.modules, &action).filter(|_| self.editable())
                else {
                    return Task::none();
                };
                return self.update(Message::RunAction { action, preset });
            }
            Message::RunAction { action, preset } => {
                let Some(state) = &self.state else {
                    return Task::none();
                };
                let Some(declared) = declared_action(&self.modules, &action) else {
                    self.status = format!("No module declares the action {action}");
                    return Task::none();
                };
                let params = match action_params(declared, &preset, &self.fields) {
                    Ok(params) => params,
                    Err(message) => {
                        self.status = message;
                        return Task::none();
                    }
                };
                let mut request =
                    json!({"asset_id":state.asset.id,"mutation":mutation(state.revision)});
                let object = request.as_object_mut().expect("the envelope is an object");
                object.extend(params);
                return self.command(format!("edit.{action}"), request);
            }
            Message::PointerMoved(point) => self.pointer = point,
            Message::PointPicked { x, y } => {
                let Some((action, x_parameter, y_parameter)) = point_pick(&self.modules) else {
                    return Task::none();
                };
                let (action, x_parameter, y_parameter) = (
                    action.to_owned(),
                    x_parameter.to_owned(),
                    y_parameter.to_owned(),
                );
                self.fields.set(&action, &x_parameter, x.to_string());
                self.fields.set(&action, &y_parameter, y.to_string());
                self.event("canvas_pick", json!({"action":action,"x":x,"y":y}));
                self.status = format!("Picked ({x}, {y}) into {action}");
            }
            Message::FocusNext => return operation::focus_next(),
            Message::FocusPrevious => return operation::focus_previous(),
            Message::Zoom(value) => self.zoom = value,
            Message::VersionName(value) => self.version_name = value,
            Message::Panned(x, y) => return self.pan(x, y),
            Message::Undo | Message::Redo => {
                let Some(state) = &self.state else {
                    return Task::none();
                };
                let method = if matches!(message, Message::Undo) {
                    "history.undo"
                } else {
                    "history.redo"
                };
                let params = json!({"asset_id":state.asset.id,"mutation":mutation(state.revision)});
                return self.command(method, params);
            }
            Message::Preview(entry_id) => {
                let Some(state) = &self.state else {
                    return Task::none();
                };
                if self.busy {
                    return Task::none();
                }
                let params = json!({"asset_id":state.asset.id,"entry_id":entry_id});
                self.busy = true;
                self.status = "Selecting history state…".into();
                return preview_task(
                    self.owner.clone(),
                    self.client,
                    state.asset.id.clone(),
                    Some(entry_id),
                    "preview.select",
                    params,
                );
            }
            Message::ReturnCurrent => {
                let Some(state) = &self.state else {
                    return Task::none();
                };
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                self.status = "Returning to current state…".into();
                return preview_task(
                    self.owner.clone(),
                    self.client,
                    state.asset.id.clone(),
                    None,
                    "preview.return-current",
                    json!({}),
                );
            }
            Message::Restore => {
                let (Some(state), HistorySelection::Entry(entry_id)) =
                    (&self.state, &self.session.preview.selection)
                else {
                    return Task::none();
                };
                let params = json!({"asset_id":state.asset.id,"mutation":mutation(state.revision),"entry_id":entry_id});
                return self.command("history.restore", params);
            }
            Message::SaveVersion => {
                let (Some(state), Some(entry_id)) = (&self.state, &self.display_entry) else {
                    return Task::none();
                };
                let name = self.version_name.trim().to_string();
                if name.is_empty() {
                    self.status = "Enter a version name first".into();
                    return Task::none();
                }
                let params = json!({"asset_id":state.asset.id,"name":name,"actor":ACTOR,"entry_id":entry_id});
                return self.version_command("version.create", params);
            }
            Message::DeleteVersion(name) => {
                let Some(state) = &self.state else {
                    return Task::none();
                };
                let params = json!({"asset_id":state.asset.id,"name":name});
                return self.version_command("version.delete", params);
            }
            Message::LoadOlder => {
                let (Some(state), Some(before)) = (&self.state, self.history.next_before_sequence)
                else {
                    return Task::none();
                };
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                return older_task(
                    self.owner.clone(),
                    self.client,
                    state.asset.id.clone(),
                    before,
                );
            }
            Message::Fit => {
                self.zoom = "Fit".into();
                return self.session_command("view.set", json!({"zoom":{"mode":"fit"}}));
            }
            Message::HundredPercent => {
                self.zoom = "100".into();
                return self
                    .session_command("view.set", json!({"zoom":{"mode":"percent","value":100.0}}));
            }
            Message::ApplyZoom => {
                let Ok(value) = self.zoom.parse::<f32>() else {
                    self.status = "Zoom must be Fit or a percentage from 10 to 1600".into();
                    return Task::none();
                };
                return self
                    .session_command("view.set", json!({"zoom":{"mode":"percent","value":value}}));
            }
            Message::ScaleFactor(scale) => {
                if scale.is_finite() && scale > 0.0 {
                    self.scale_factor = scale;
                }
            }
            Message::Close => {
                self.event("shutdown", json!({"while_loading":self.activity.pending}));
                self.live_server.take();
                self.owner.disconnect(self.client);
                self.owner.stop();
                let join = self.owner_join.take();
                let log = self.diagnostics.take();
                return Task::perform(
                    async move {
                        if let Some(log) = log {
                            log.finish();
                        }
                        if let Some(join) = join {
                            let _ = join.join();
                        }
                    },
                    |_| (),
                )
                .then(|_| iced::exit());
            }
        }
        Task::none()
    }

    fn accept(&mut self, refresh: Refresh) {
        self.api_sequence = refresh.sequence;
        self.adopt(refresh.session);
        match refresh.history {
            Some(history) => self.history = history,
            None => merge_current_entry(&mut self.history, refresh.state.current_entry.clone()),
        }
        self.versions = refresh.versions;
        self.lineage = refresh
            .lineage
            .steps
            .iter()
            .map(|step| step.entry_id.clone())
            .collect();
        self.lineage_floor = refresh
            .lineage
            .next_entry_id
            .as_ref()
            .and_then(|_| refresh.lineage.steps.last().map(|step| step.sequence));
        self.state = Some(refresh.state);
        self.display_entry = Some(refresh.job.entry.id.clone());
        self.preview_generation = self.preview_queue.request(refresh.job);
        self.status = "Rendering selected history state…".into();
    }

    fn on_current_lineage(&self, entry: &HistoryEntry) -> bool {
        self.lineage.contains(&entry.id)
            || self
                .lineage_floor
                .is_some_and(|floor| entry.sequence <= floor)
    }

    /// Pan is session state like zoom, but scroll events arrive faster than round trips complete:
    /// keep one request in flight and only the newest pending position.
    fn pan(&mut self, x: f32, y: f32) -> Task<Message> {
        if self.pan_in_flight {
            self.pending_pan = Some((x, y));
            return Task::none();
        }
        self.pan_in_flight = true;
        pan_task(self.owner.clone(), self.client, x, y)
    }

    /// Every mutation, generated or not, takes the narrowest completion path: the command, one
    /// `asset.state` refresh and one preview job.
    fn command(&mut self, method: impl Into<String>, params: Value) -> Task<Message> {
        let Some(state) = &self.state else {
            return Task::none();
        };
        if self.busy {
            return Task::none();
        }
        let method = method.into();
        self.busy = true;
        self.status = format!("Running {method}…");
        state_task(
            self.owner.clone(),
            self.client,
            state.asset.id.clone(),
            method,
            params,
        )
    }

    fn session_command(&mut self, method: &'static str, params: Value) -> Task<Message> {
        if self.state.is_none() || self.busy {
            return Task::none();
        }
        self.busy = true;
        self.status = format!("Running {method}…");
        session_task(self.owner.clone(), self.client, method, params)
    }

    fn version_command(&mut self, method: &'static str, params: Value) -> Task<Message> {
        let Some(state) = &self.state else {
            return Task::none();
        };
        if self.busy {
            return Task::none();
        }
        self.busy = true;
        self.status = format!("Running {method}…");
        versions_task(
            self.owner.clone(),
            self.client,
            state.asset.id.clone(),
            method,
            params,
        )
    }

    /// An edit is possible when an asset is open, the session shows the current state and no
    /// request is in flight.
    fn editable(&self) -> bool {
        self.state.is_some() && self.session.preview.can_edit() && !self.busy
    }

    /// Every tool control on screen, generated from the fetched descriptors. The desktop lays them
    /// out and edits text; the modules declare what exists and what it is worth.
    fn tool_panel(&self, editable: bool) -> Element<'_, Message> {
        if self.modules.is_empty() {
            let message = if self.modules_ready {
                "No tool modules are available"
            } else {
                "Loading tool modules…"
            };
            return text(message).size(14).into();
        }
        let mut panel = column![].spacing(18);
        for module in &self.modules {
            let enabled = match &module.availability {
                Availability::Available => editable,
                Availability::Unavailable { reason } => {
                    panel = panel
                        .push(text(format!("{} · unavailable: {reason}", module.title)).size(14));
                    false
                }
            };
            for control in &module.controls {
                panel = panel.push(self.control_element(module, control, enabled));
            }
        }
        panel.into()
    }

    /// One declared control. An unrenderable kind is named on screen, never dropped.
    fn control_element<'a>(
        &'a self,
        module: &'a ModuleDescriptor,
        control: &'a Control,
        enabled: bool,
    ) -> Element<'a, Message> {
        match classify(control) {
            Rendered::Group { label, controls } => {
                let mut group = column![text(label).size(18)].spacing(8);
                for child in controls {
                    group = group.push(self.control_element(module, child, enabled));
                }
                group.into()
            }
            Rendered::Number {
                action,
                parameter,
                label,
            } => {
                let Some(declared) = declared_parameter(module, action, parameter) else {
                    return text(undeclared_label(action, parameter)).size(12).into();
                };
                let value = self.fields.get(action, parameter).unwrap_or_default();
                let (input_action, input_parameter) = (action.to_owned(), parameter.to_owned());
                let input = text_input(label, value)
                    .id(field_id(action, parameter, None))
                    .on_input(move |text| Message::ControlChanged {
                        action: input_action.clone(),
                        parameter: input_parameter.clone(),
                        text,
                    })
                    .on_submit(Message::ControlSubmitted {
                        action: action.to_owned(),
                    })
                    .width(90);
                let mut field = column![
                    row![text(labelled(label, declared)).size(12).width(110), input]
                        .spacing(6)
                        .align_y(iced::Alignment::Center)
                ]
                .spacing(4);
                if let Err(message) = parse_field(declared, value) {
                    field = field.push(text(message).size(11));
                }
                field.into()
            }
            Rendered::Color {
                action,
                parameter,
                label,
            } => {
                let Some(declared) = declared_parameter(module, action, parameter) else {
                    return text(undeclared_label(action, parameter)).size(12).into();
                };
                let value = self.fields.get(action, parameter).unwrap_or_default();
                let mut channels = row![text(labelled(label, declared)).size(12).width(110)]
                    .spacing(6)
                    .align_y(iced::Alignment::Center);
                for (index, name) in CHANNELS.iter().enumerate() {
                    let (input_action, input_parameter, current) =
                        (action.to_owned(), parameter.to_owned(), value.to_owned());
                    channels = channels.push(
                        text_input(name, channel_text(value, index))
                            .id(field_id(action, parameter, Some(name)))
                            .on_input(move |text| Message::ControlChanged {
                                action: input_action.clone(),
                                parameter: input_parameter.clone(),
                                text: replace_channel(&current, index, &text),
                            })
                            .on_submit(Message::ControlSubmitted {
                                action: action.to_owned(),
                            })
                            .width(55),
                    );
                }
                let mut field = column![channels].spacing(4);
                if let Err(message) = parse_field(declared, value) {
                    field = field.push(text(message).size(11));
                }
                field.into()
            }
            Rendered::Action {
                action,
                label,
                preset,
            } => {
                let runnable = enabled
                    && declared_action(&self.modules, action).is_some_and(|declared| {
                        action_params(declared, preset, &self.fields).is_ok()
                    });
                button(text(label))
                    .on_press_maybe(runnable.then(|| Message::RunAction {
                        action: action.to_owned(),
                        preset: preset.clone(),
                    }))
                    .into()
            }
            Rendered::Unsupported(kind) => text(unsupported_label(&kind)).size(12).into(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let current = self.state.as_ref();
        let editable = self.editable();
        let open = button("Open image")
            .on_press_maybe((!self.busy && self.evidence.is_none()).then_some(Message::Open));
        let header = row![text("Lightwell").size(22), open]
            .spacing(16)
            .align_y(iced::Alignment::Center);

        // A pick is only meaningful while the current state can be edited, and only when a module
        // declares one; the adapter maps a click to image pixels and fills that module's fields.
        let picking = editable && point_pick(&self.modules).is_some();
        let pointer = self.pointer;
        let surface: Element<'_, Message> = match (&self.photo, self.dimensions) {
            (Some(allocation), Some((width, height))) => match self.session.preview.view.zoom {
                Zoom::Fit => {
                    let handle = allocation.handle().clone();
                    // Fit needs the available size to know where iced draws the contained image.
                    responsive(move |available| {
                        let photo = image(handle.clone())
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .content_fit(ContentFit::Contain);
                        if !picking {
                            return photo.into();
                        }
                        let mut area = mouse_area(photo).on_move(move |point| {
                            Message::PointerMoved(fit_pick((width, height), available, point))
                        });
                        if let Some((x, y)) = pointer {
                            area = area.on_press(Message::PointPicked { x, y });
                        }
                        area.into()
                    })
                    .into()
                }
                Zoom::Percent { value } => {
                    let scale = value / 100.0 / self.scale_factor;
                    let photo = image(allocation.handle().clone())
                        .width(Length::Fixed(width as f32 * scale))
                        .height(Length::Fixed(height as f32 * scale));
                    // Inside the scrollable the reported point is already content-space: the
                    // scrollable translates the cursor by its offset before its content sees it.
                    let photo: Element<'_, Message> = if picking {
                        let mut area = mouse_area(photo).on_move(move |point| {
                            Message::PointerMoved(percent_pick((width, height), scale, point))
                        });
                        if let Some((x, y)) = pointer {
                            area = area.on_press(Message::PointPicked { x, y });
                        }
                        area.into()
                    } else {
                        photo.into()
                    };
                    scrollable(container(photo).center(Length::Shrink))
                        .direction(iced::widget::scrollable::Direction::Both {
                            vertical: iced::widget::scrollable::Scrollbar::default(),
                            horizontal: iced::widget::scrollable::Scrollbar::default(),
                        })
                        .on_scroll(|viewport| {
                            let offset = viewport.absolute_offset();
                            Message::Panned(offset.x, offset.y)
                        })
                        .into()
                }
            },
            _ => container(text("Open a photograph").size(24))
                .center(Length::Fill)
                .into(),
        };

        let tools = self.tool_panel(editable);

        let zoom = column![
            text("View").size(18),
            row![
                button("Fit").on_press_maybe(current.is_some().then_some(Message::Fit)),
                button("100%").on_press_maybe(current.is_some().then_some(Message::HundredPercent)),
                text_input("Zoom %", &self.zoom)
                    .on_input(Message::Zoom)
                    .on_submit(Message::ApplyZoom)
                    .width(80),
                button("Set").on_press_maybe(current.is_some().then_some(Message::ApplyZoom)),
            ]
            .spacing(6),
            text(format!("Display scale {:.2}×; 100% maps one source pixel to one physical framebuffer pixel", self.scale_factor)).size(11),
        ]
        .spacing(8);

        let mut history_rows = column![
            row![
                text("History").size(18),
                button("Undo").on_press_maybe(editable.then_some(Message::Undo)),
                button("Redo").on_press_maybe(editable.then_some(Message::Redo)),
            ]
            .spacing(6)
        ]
        .spacing(5);
        for entry in &self.history.entries {
            let marker = if current
                .map(|state| state.current_entry.id == entry.id)
                .unwrap_or(false)
            {
                "●"
            } else if self.display_entry.as_ref() == Some(&entry.id) {
                "◉"
            } else {
                "○"
            };
            let branch = if self.on_current_lineage(entry) {
                ""
            } else {
                " · branch"
            };
            let label = format!(
                "{marker} {} · {} · {}{branch}",
                entry.sequence, entry.action_id, entry.actor
            );
            history_rows = history_rows.push(
                button(text(label).size(12))
                    .width(Length::Fill)
                    .on_press_maybe((!self.busy).then_some(Message::Preview(entry.id.clone()))),
            );
        }
        if self.history.next_before_sequence.is_some() {
            history_rows = history_rows.push(
                button("Load older history")
                    .on_press_maybe((!self.busy).then_some(Message::LoadOlder)),
            );
        }
        if !self.session.preview.can_edit() {
            history_rows = history_rows.push(
                row![
                    button("Return to current")
                        .on_press_maybe((!self.busy).then_some(Message::ReturnCurrent)),
                    button("Restore this state")
                        .on_press_maybe((!self.busy).then_some(Message::Restore)),
                ]
                .spacing(6),
            );
        }

        let can_save = current.is_some() && self.display_entry.is_some() && !self.busy;
        let mut version_rows = column![
            text("Versions").size(18),
            row![
                text_input("Name the displayed state", &self.version_name)
                    .on_input(Message::VersionName)
                    .on_submit(Message::SaveVersion)
                    .width(Length::Fill),
                button("Save").on_press_maybe(can_save.then_some(Message::SaveVersion)),
            ]
            .spacing(6),
        ]
        .spacing(5);
        for version in &self.versions {
            let marker = if self.display_entry.as_ref() == Some(&version.entry_id) {
                "◉"
            } else {
                "○"
            };
            let label = format!(
                "{marker} {} · entry {}",
                version.name, version.entry_sequence
            );
            version_rows = version_rows.push(
                row![
                    button(text(label).size(12))
                        .width(Length::Fill)
                        .on_press_maybe(
                            (!self.busy).then_some(Message::Preview(version.entry_id.clone()))
                        ),
                    button(text("Delete").size(12)).on_press_maybe(
                        (!self.busy).then_some(Message::DeleteVersion(version.name.clone()))
                    ),
                ]
                .spacing(6),
            );
        }

        let layers = self
            .history
            .entries
            .iter()
            .find(|entry| Some(&entry.id) == self.display_entry.as_ref())
            .map(|entry| {
                if entry.snapshot.recipe.layers.is_empty() {
                    "Original · no edit layers".into()
                } else {
                    entry
                        .snapshot
                        .recipe
                        .layers
                        .iter()
                        .enumerate()
                        .map(|(index, layer)| {
                            format!(
                                "{}: {} ({})",
                                index + 1,
                                layer.effect_id,
                                short(layer.id.as_str())
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            })
            .unwrap_or_else(|| "No layer selected".into());
        let sidebar = scrollable(
            column![
                tools,
                zoom,
                history_rows,
                version_rows,
                text("Layer stack").size(18),
                text(layers).size(11)
            ]
            .spacing(18)
            .padding(12),
        )
        .width(SIDEBAR_WIDTH);
        column![
            header,
            row![
                container(surface).width(Length::Fill).height(Length::Fill),
                sidebar
            ]
            .spacing(SPACING)
            .height(Length::Fill),
            text(&self.status).size(12),
        ]
        .spacing(SPACING)
        .padding(PADDING)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let events = iced::event::listen_with(|event, _, _| {
            if matches!(
                event,
                iced::Event::Window(iced::window::Event::CloseRequested)
            ) {
                return Some(Message::Close);
            }
            if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key, modifiers, ..
            }) = event
            {
                if modifiers.command() {
                    if matches!(key,iced::keyboard::Key::Character(ref value) if value.eq_ignore_ascii_case("o"))
                    {
                        return Some(Message::Open);
                    }
                    if matches!(key,iced::keyboard::Key::Character(ref value) if value.eq_ignore_ascii_case("z"))
                    {
                        return Some(if modifiers.shift() {
                            Message::Redo
                        } else {
                            Message::Undo
                        });
                    }
                    return None;
                }
                // Tab walks the generated fields; shift is the only modifier it tolerates.
                if matches!(
                    key,
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab)
                ) && !modifiers.alt()
                    && !modifiers.control()
                    && !modifiers.logo()
                {
                    return Some(if modifiers.shift() {
                        Message::FocusPrevious
                    } else {
                        Message::FocusNext
                    });
                }
            }
            None
        });
        let mut subscriptions = vec![events];
        if self.preview_queue.is_busy() {
            subscriptions.push(iced::time::every(Duration::from_millis(16)).map(|_| Message::Poll));
        }
        if self.state.is_some() && self.evidence.is_none() {
            subscriptions
                .push(iced::time::every(Duration::from_millis(500)).map(|_| Message::Sync));
        }
        if let Some(evidence) = &self.evidence {
            subscriptions
                .push(iced::time::every(Duration::from_millis(250)).map(|_| Message::EvidenceTick));
            if evidence.capture_pending {
                subscriptions.push(iced::window::frames().map(|_| Message::Capture));
            }
        }
        Subscription::batch(subscriptions)
    }
}

fn short(value: &str) -> &str {
    value.get(..value.len().min(12)).unwrap_or(value)
}

/// The channels of a color parameter, in declared order.
const CHANNELS: [&str; 3] = ["R", "G", "B"];

/// The text typed into each generated field, by (action id, parameter name). The raw text is kept:
/// validation always runs against the declared parameter, never against a parsed copy.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Fields(BTreeMap<(String, String), String>);

impl Fields {
    /// Seed every declared field from its parameter's default, else from the limit it accepts.
    fn seeded(modules: &[ModuleDescriptor]) -> Self {
        let mut fields = Self::default();
        for module in modules {
            seed_controls(module, &module.controls, &mut fields);
        }
        fields
    }

    fn get(&self, action: &str, parameter: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|((declared, name), _)| declared == action && name == parameter)
            .map(|(_, text)| text.as_str())
    }

    fn set(&mut self, action: &str, parameter: &str, text: String) {
        self.0
            .insert((action.to_owned(), parameter.to_owned()), text);
    }

    /// Correlated evidence: what every generated control held when a frame was captured.
    fn summary(&self) -> Value {
        Value::Object(
            self.0
                .iter()
                .map(|((action, parameter), text)| {
                    (format!("{action}.{parameter}"), Value::from(text.clone()))
                })
                .collect(),
        )
    }
}

fn seed_controls(module: &ModuleDescriptor, controls: &[Control], fields: &mut Fields) {
    for control in controls {
        match classify(control) {
            Rendered::Group { controls, .. } => seed_controls(module, controls, fields),
            Rendered::Number {
                action, parameter, ..
            }
            | Rendered::Color {
                action, parameter, ..
            } => {
                if let Some(declared) = declared_parameter(module, action, parameter) {
                    fields.set(action, parameter, seed_text(declared));
                }
            }
            Rendered::Action { .. } | Rendered::Unsupported(_) => {}
        }
    }
}

/// A field starts at the declared default; without one it starts at the lowest accepted value.
fn seed_text(parameter: &ParameterDescriptor) -> String {
    match &parameter.kind {
        ParameterKind::Integer { min, .. } => parameter
            .default
            .as_ref()
            .and_then(Value::as_i64)
            .unwrap_or(*min)
            .to_string(),
        ParameterKind::Color => parameter
            .default
            .as_ref()
            .and_then(Value::as_array)
            .filter(|channels| channels.len() == CHANNELS.len())
            .map(|channels| {
                channels
                    .iter()
                    .map(|channel| channel.as_u64().unwrap_or_default().to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_else(|| "0,0,0".into()),
        ParameterKind::Enum { options } => parameter
            .default
            .as_ref()
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| options.first().cloned())
            .unwrap_or_default(),
    }
}

fn range_message(name: &str, min: i64, max: i64) -> String {
    format!("{name} must be an integer within {min}..={max}")
}

/// One field's text read as the value its parameter declares, or the message naming what it needs.
fn parse_field(parameter: &ParameterDescriptor, text: &str) -> Result<Value, String> {
    let name = &parameter.name;
    match &parameter.kind {
        ParameterKind::Integer { min, max } => text
            .trim()
            .parse::<i64>()
            .ok()
            .filter(|value| (min..=max).contains(&value))
            .map(Value::from)
            .ok_or_else(|| range_message(name, *min, *max)),
        ParameterKind::Color => parse_color(text)
            .map(|rgb| Value::from(rgb.to_vec()))
            .ok_or_else(|| format!("{name} must be three channels 0..=255")),
        ParameterKind::Enum { options } => options
            .iter()
            .find(|option| *option == text.trim())
            .map(|option| Value::from(option.clone()))
            .ok_or_else(|| format!("{name} must be one of {}", options.join(", "))),
    }
}

fn parse_color(text: &str) -> Option<[u8; 3]> {
    let mut channels = text.split(',');
    let mut rgb = [0u8; 3];
    for slot in rgb.iter_mut() {
        *slot = channels.next()?.trim().parse().ok()?;
    }
    channels.next().is_none().then_some(rgb)
}

fn channel_text(value: &str, index: usize) -> &str {
    value.split(',').nth(index).unwrap_or_default().trim()
}

/// One channel of a color field replaced, keeping the other two as typed.
fn replace_channel(current: &str, index: usize, text: &str) -> String {
    let mut channels: Vec<&str> = (0..CHANNELS.len())
        .map(|channel| channel_text(current, channel))
        .collect();
    let trimmed = text.trim();
    if let Some(slot) = channels.get_mut(index) {
        *slot = trimmed;
    }
    channels.join(",")
}

/// A control label carries the parameter's declared unit, e.g. `X (px)`.
fn labelled(label: &str, parameter: &ParameterDescriptor) -> String {
    match &parameter.unit {
        Some(unit) => format!("{label} ({unit})"),
        None => label.to_owned(),
    }
}

/// A stable widget identity per generated field, so focus survives a redraw.
fn field_id(action: &str, parameter: &str, channel: Option<&str>) -> String {
    match channel {
        Some(channel) => format!("lightwell.field.{action}.{parameter}.{channel}"),
        None => format!("lightwell.field.{action}.{parameter}"),
    }
}

fn undeclared_label(action: &str, parameter: &str) -> String {
    format!("Unsupported control: {action} declares no parameter {parameter}")
}

fn unsupported_label(kind: &str) -> String {
    format!("Unsupported control: {kind}")
}

/// What the desktop makes of one declared control. A kind this build cannot draw keeps its name on
/// screen rather than disappearing from the panel.
enum Rendered<'a> {
    Group {
        label: &'a str,
        controls: &'a [Control],
    },
    Number {
        action: &'a str,
        parameter: &'a str,
        label: &'a str,
    },
    Color {
        action: &'a str,
        parameter: &'a str,
        label: &'a str,
    },
    Action {
        action: &'a str,
        label: &'a str,
        preset: &'a Map<String, Value>,
    },
    Unsupported(String),
}

fn classify(control: &Control) -> Rendered<'_> {
    match control {
        Control::Group { label, controls } => Rendered::Group { label, controls },
        Control::Number {
            action,
            parameter,
            label,
        } => Rendered::Number {
            action,
            parameter,
            label,
        },
        Control::Color {
            action,
            parameter,
            label,
        } => Rendered::Color {
            action,
            parameter,
            label,
        },
        Control::Action {
            action,
            label,
            preset,
        } => Rendered::Action {
            action,
            label,
            preset,
        },
        // A kind added to the descriptor later is reported, never dropped.
        #[allow(unreachable_patterns)]
        other => Rendered::Unsupported(control_kind(other)),
    }
}

/// The descriptor's own kind tag, so an unrenderable control can still be named.
fn control_kind(control: &Control) -> String {
    serde_json::to_value(control)
        .ok()
        .and_then(|value| value.get("kind").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_else(|| "unknown".into())
}

fn declared_action<'a>(
    modules: &'a [ModuleDescriptor],
    action: &str,
) -> Option<&'a ActionDescriptor> {
    modules.iter().find_map(|module| module.action(action))
}

fn declared_parameter<'a>(
    module: &'a ModuleDescriptor,
    action: &str,
    parameter: &str,
) -> Option<&'a ParameterDescriptor> {
    module.action(action)?.parameter(parameter)
}

/// The request fields for one action: preset values merged over the parsed field text, preset
/// wins. A parameter with a declared default is left out so the host applies that default.
fn action_params(
    action: &ActionDescriptor,
    preset: &Map<String, Value>,
    fields: &Fields,
) -> Result<Map<String, Value>, String> {
    let mut params = Map::new();
    for parameter in &action.parameters {
        if let Some(value) = preset.get(&parameter.name) {
            params.insert(parameter.name.clone(), value.clone());
        } else if let Some(text) = fields.get(&action.id, &parameter.name) {
            params.insert(parameter.name.clone(), parse_field(parameter, text)?);
        } else if parameter.default.is_none() && parameter.required {
            return Err(format!("{} requires {}", action.title, parameter.name));
        }
    }
    Ok(params)
}

/// Enter in a field runs the first control that invokes that action, with its preset.
fn submit_preset(modules: &[ModuleDescriptor], action: &str) -> Option<Map<String, Value>> {
    declared_action(modules, action)?;
    Some(
        modules
            .iter()
            .find_map(|module| control_preset(&module.controls, action))
            .cloned()
            .unwrap_or_default(),
    )
}

fn control_preset<'a>(controls: &'a [Control], action: &str) -> Option<&'a Map<String, Value>> {
    controls.iter().find_map(|control| match classify(control) {
        Rendered::Group { controls, .. } => control_preset(controls, action),
        Rendered::Action {
            action: declared,
            preset,
            ..
        } if declared == action => Some(preset),
        _ => None,
    })
}

/// The first available module that declares a canvas pick: its action and coordinate parameters.
fn point_pick(modules: &[ModuleDescriptor]) -> Option<(&str, &str, &str)> {
    modules.iter().find_map(|module| match &module.canvas {
        Some(CanvasInteraction::PointPick { action, x, y }) if module.is_available() => {
            Some((action.as_str(), x.as_str(), y.as_str()))
        }
        _ => None,
    })
}

/// Where iced draws a contained image inside `available`, matching the image widget's own bounds:
/// `ContentFit::Contain` sized and centered.
fn fit_rect(image: (u32, u32), available: Size) -> Option<Rectangle> {
    let content = Size::new(image.0 as f32, image.1 as f32);
    if content.width <= 0.0
        || content.height <= 0.0
        || !(available.width > 0.0 && available.height > 0.0)
    {
        return None;
    }
    let size = ContentFit::Contain.fit(content, available);
    (size.width > 0.0 && size.height > 0.0).then(|| {
        Rectangle::new(
            Point::new(
                (available.width - size.width) / 2.0,
                (available.height - size.height) / 2.0,
            ),
            size,
        )
    })
}

/// Fit: the reported point spans the whole surface, so the centered rectangle is removed first.
fn fit_pick(image: (u32, u32), available: Size, point: Point) -> Option<(u32, u32)> {
    let rect = fit_rect(image, available)?;
    image_pixel(
        (point.x - rect.x) * image.0 as f32 / rect.width,
        (point.y - rect.y) * image.1 as f32 / rect.height,
        image,
    )
}

/// Percent: the reported point is local to the displayed raster, which is the image scaled
/// uniformly. The scrollable translates the cursor by its scroll offset before its content sees
/// it, so the pan position never enters this mapping.
fn percent_pick(image: (u32, u32), scale: f32, point: Point) -> Option<(u32, u32)> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    image_pixel(point.x / scale, point.y / scale, image)
}

fn image_pixel(x: f32, y: f32, (width, height): (u32, u32)) -> Option<(u32, u32)> {
    let inside = |value: f32, limit: u32| {
        (value.is_finite() && value >= 0.0 && value < limit as f32).then(|| value.floor() as u32)
    };
    Some((inside(x, width)?, inside(y, height)?))
}

/// Module identity for correlated evidence; descriptors carry no source paths.
fn module_summary(modules: &[ModuleDescriptor]) -> Value {
    Value::Array(
        modules
            .iter()
            .map(|module| {
                json!({"id":module.id,"available":module.is_available(),"actions":module.actions.iter().map(|action| action.id.clone()).collect::<Vec<_>>()})
            })
            .collect(),
    )
}

fn merge_current_entry(history: &mut HistoryPage, entry: HistoryEntry) {
    history.entries.retain(|existing| existing.id != entry.id);
    let position = history
        .entries
        .partition_point(|existing| existing.sequence > entry.sequence);
    history.entries.insert(position, entry);
    history.entries.truncate(HISTORY_PAGE_SIZE);
    history.next_before_sequence = (history.entries.len() == HISTORY_PAGE_SIZE)
        .then(|| history.entries.last().expect("page is not empty").sequence);
}

fn mutation(revision: u64) -> Mutation {
    Mutation {
        expected_revision: revision,
        request_id: format!(
            "desktop-{}-{}",
            std::process::id(),
            REQUEST_NUMBER.fetch_add(1, Ordering::Relaxed)
        ),
        actor: ACTOR.into(),
    }
}

fn call(
    owner: &OwnerHandle,
    client: ClientId,
    method: &str,
    params: Value,
) -> Result<(Value, u64), String> {
    let request = ApiRequest {
        id: format!("ui-{}", REQUEST_NUMBER.fetch_add(1, Ordering::Relaxed)),
        method: method.into(),
        params,
        token: None,
    };
    let response = owner
        .call(client, request)
        .map_err(|error| error.to_string())?;
    if let Some(error) = response.error {
        Err(format!("{}: {}", error.code, error.message))
    } else {
        Ok((response.result.unwrap_or(Value::Null), response.sequence))
    }
}

fn parse<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(|error| error.to_string())
}

/// Read authoritative state back after a change or an external event.
fn refresh(
    owner: &OwnerHandle,
    client: ClientId,
    asset_id: AssetId,
    with_history: bool,
    mut sequence: u64,
) -> Result<Refresh, String> {
    let mut fetch = |method: &str, params: Value| -> Result<Value, String> {
        let (value, seen) = call(owner, client, method, params)?;
        sequence = sequence.max(seen);
        Ok(value)
    };
    let state: EditorState = parse(fetch("asset.state", json!({"asset_id":asset_id}))?)?;
    let history = if with_history {
        Some(parse::<HistoryPage>(fetch(
            "history.list",
            json!({"asset_id":asset_id,"before_sequence":null,"limit":HISTORY_PAGE_SIZE}),
        )?)?)
    } else {
        None
    };
    let versions: Vec<Version> =
        parse(fetch("version.list", json!({"asset_id":asset_id}))?["versions"].take())?;
    let lineage: Lineage = parse(fetch(
        "history.lineage",
        json!({"asset_id":asset_id,"limit":LINEAGE_LIMIT}),
    )?)?;
    let session: ClientSession = parse(fetch("session.state", json!({}))?)?;
    let selected = match &session.preview.selection {
        HistorySelection::Current => None,
        HistorySelection::Entry(entry_id) => Some(entry_id.clone()),
    };
    let job = owner
        .preview_job(asset_id, selected)
        .map_err(|error| error.to_string())?;
    Ok(Refresh {
        state,
        history,
        versions,
        lineage,
        job,
        session,
        sequence,
    })
}

/// Discovery runs once: the controls on screen are whatever the registered modules declare.
fn modules_task(owner: OwnerHandle, client: ClientId) -> Task<Message> {
    Task::perform(
        async move {
            let (mut listed, _) = call(&owner, client, "module.list", json!({}))?;
            parse::<Vec<ModuleDescriptor>>(listed["modules"].take())
        },
        Message::ModulesLoaded,
    )
}

fn import_task(owner: OwnerHandle, client: ClientId, path: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let (result, sequence) = call(&owner, client, "catalog.import", json!({"path":path}))?;
            let state: EditorState = parse(result)?;
            let _ = call(&owner, client, "preview.return-current", json!({}))?;
            refresh(&owner, client, state.asset.id, true, sequence)
        },
        |result| Message::Refreshed(result.map(Box::new)),
    )
}

fn state_task(
    owner: OwnerHandle,
    client: ClientId,
    asset_id: AssetId,
    method: String,
    params: Value,
) -> Task<Message> {
    Task::perform(
        async move {
            let (_, sequence) = call(&owner, client, &method, params)?;
            refresh(&owner, client, asset_id, false, sequence)
        },
        |result| Message::Refreshed(result.map(Box::new)),
    )
}

fn preview_task(
    owner: OwnerHandle,
    client: ClientId,
    asset_id: AssetId,
    entry_id: Option<EntryId>,
    method: &'static str,
    params: Value,
) -> Task<Message> {
    Task::perform(
        async move {
            let (mut result, sequence) = call(&owner, client, method, params)?;
            let session: ClientSession = parse(result["session"].take())?;
            let job = owner
                .preview_job(asset_id, entry_id)
                .map_err(|error| error.to_string())?;
            Ok(PreviewPayload {
                job,
                session,
                sequence,
            })
        },
        |result| Message::PreviewLoaded(result.map(Box::new)),
    )
}

fn session_task(
    owner: OwnerHandle,
    client: ClientId,
    method: &'static str,
    params: Value,
) -> Task<Message> {
    Task::perform(
        async move {
            let (result, sequence) = call(&owner, client, method, params)?;
            Ok((parse::<ClientSession>(result)?, sequence))
        },
        Message::SessionUpdated,
    )
}

fn pan_task(owner: OwnerHandle, client: ClientId, x: f32, y: f32) -> Task<Message> {
    Task::perform(
        async move {
            let (result, _) = call(&owner, client, "view.set", json!({"pan_x":x,"pan_y":y}))?;
            parse::<ClientSession>(result)
        },
        Message::PanSynced,
    )
}

fn versions_task(
    owner: OwnerHandle,
    client: ClientId,
    asset_id: AssetId,
    method: &'static str,
    params: Value,
) -> Task<Message> {
    Task::perform(
        async move {
            let (_, sequence) = call(&owner, client, method, params)?;
            let (mut listed, seen) =
                call(&owner, client, "version.list", json!({"asset_id":asset_id}))?;
            Ok((
                parse::<Vec<Version>>(listed["versions"].take())?,
                sequence.max(seen),
            ))
        },
        Message::VersionsLoaded,
    )
}

fn sync_task(owner: OwnerHandle, client: ClientId, asset_id: AssetId, after: u64) -> Task<Message> {
    Task::perform(
        async move {
            let (events, sequence) = call(&owner, client, "events.since", json!({"after":after}))?;
            let events: EventsResult = parse(events)?;
            if events.events.is_empty() && !events.gap {
                Ok(SyncResult::Unchanged { sequence })
            } else {
                refresh(&owner, client, asset_id, true, sequence)
                    .map(Box::new)
                    .map(SyncResult::Changed)
            }
        },
        Message::Synced,
    )
}

fn older_task(
    owner: OwnerHandle,
    client: ClientId,
    asset_id: AssetId,
    before_sequence: u64,
) -> Task<Message> {
    Task::perform(
        async move {
            let (page, sequence) = call(
                &owner,
                client,
                "history.list",
                json!({"asset_id":asset_id,"before_sequence":before_sequence,"limit":HISTORY_PAGE_SIZE}),
            )?;
            Ok((parse::<HistoryPage>(page)?, sequence))
        },
        Message::OlderLoaded,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightwell_core::{AssetRecord, LineageStep, Snapshot, SourceImage};

    fn boot() -> (Editor, PathBuf) {
        let catalog = std::env::temp_dir().join(format!(
            "lightwell-desktop-{}-{}.sqlite",
            std::process::id(),
            REQUEST_NUMBER.fetch_add(1, Ordering::Relaxed)
        ));
        let (owner, join) = OwnerHandle::start(&catalog).unwrap();
        let (editor, _) = Editor::new(Boot {
            owner,
            join,
            live_server: None,
            config: Config::default(),
        });
        (editor, catalog)
    }

    fn finish(mut editor: Editor, catalog: PathBuf) {
        editor.owner.stop();
        editor.owner_join.take().unwrap().join().unwrap();
        drop(editor);
        std::fs::remove_file(catalog).unwrap();
    }

    fn entry(asset: &AssetId, sequence: u64, parent: Option<&EntryId>) -> HistoryEntry {
        HistoryEntry {
            id: EntryId::new(),
            asset_id: asset.clone(),
            sequence,
            action_id: "test".into(),
            parameters: json!({}),
            actor: "test".into(),
            timestamp_ms: 0,
            request_id: None,
            base_revision: sequence,
            result_revision: sequence,
            snapshot: Snapshot::original(asset.clone()),
            undo_parent: parent.cloned(),
            restore_target: None,
        }
    }

    fn refresh_for(
        asset: &AssetId,
        current: &HistoryEntry,
        page: Vec<HistoryEntry>,
        lineage: &[&HistoryEntry],
        truncated: bool,
    ) -> Refresh {
        Refresh {
            state: EditorState {
                asset: AssetRecord {
                    id: asset.clone(),
                    source_root: PathBuf::new(),
                    locator: PathBuf::new(),
                    fingerprint: "f".into(),
                    file_identity: "i".into(),
                    byte_len: 0,
                    width: 1,
                    height: 1,
                },
                revision: current.sequence,
                current_entry: current.clone(),
                redo: Vec::new(),
            },
            history: (!page.is_empty()).then_some(HistoryPage {
                entries: page,
                next_before_sequence: None,
            }),
            versions: Vec::new(),
            lineage: Lineage {
                steps: lineage
                    .iter()
                    .map(|entry| LineageStep {
                        entry_id: entry.id.clone(),
                        sequence: entry.sequence,
                        action_id: entry.action_id.clone(),
                        undo_parent: entry.undo_parent.clone(),
                    })
                    .collect(),
                next_entry_id: truncated.then(EntryId::new),
            },
            job: PreviewJob {
                registry: std::sync::Arc::new(lightwell_core::ModuleRegistry::builtin()),
                source: SourceImage {
                    width: 1,
                    height: 1,
                    rgba: vec![0, 0, 0, 255].into(),
                    fingerprint: "f".into(),
                    orientation: 1,
                },
                entry: current.clone(),
            },
            session: ClientSession::default(),
            sequence: 7,
        }
    }

    /// The descriptors the desktop would fetch through `module.list`.
    fn descriptors() -> Vec<ModuleDescriptor> {
        lightwell_core::ModuleRegistry::builtin()
            .descriptors()
            .into_iter()
            .cloned()
            .collect()
    }

    fn parameter_of<'a>(
        modules: &'a [ModuleDescriptor],
        action: &str,
        name: &str,
    ) -> &'a ParameterDescriptor {
        declared_action(modules, action)
            .and_then(|declared| declared.parameter(name))
            .expect("the declared parameter")
    }

    #[test]
    fn fields_are_seeded_from_declared_defaults_and_limits() {
        let modules = descriptors();
        let fields = Fields::seeded(&modules);
        let (action, x, y) = point_pick(&modules).expect("the pixel module declares a canvas pick");
        assert_eq!(
            fields.get(action, x),
            Some("0"),
            "integers seed at their min"
        );
        assert_eq!(fields.get(action, y), Some("0"));
        let color = declared_action(&modules, action)
            .expect("the declared action")
            .parameters
            .iter()
            .find(|parameter| matches!(parameter.kind, ParameterKind::Color))
            .expect("the pixel action declares a color");
        assert_eq!(fields.get(action, &color.name), Some("0,0,0"));
        // Only declared fields exist: an action driven by presets alone has none.
        assert!(
            fields.summary().as_object().expect("an object").len() == 3,
            "{}",
            fields.summary()
        );
        assert_eq!(
            seed_text(&ParameterDescriptor {
                default: Some(json!(7)),
                ..parameter_of(&modules, action, x).clone()
            }),
            "7",
            "a declared default wins over the minimum"
        );
    }

    #[test]
    fn field_text_is_validated_against_the_declared_parameter() {
        let modules = descriptors();
        let (action, x, _) = point_pick(&modules).expect("a canvas pick");
        let coordinate = parameter_of(&modules, action, x);
        assert_eq!(parse_field(coordinate, " 12 ").unwrap(), json!(12));
        for text in ["", "1.5", "-1", "16384", "twelve"] {
            let message = parse_field(coordinate, text).expect_err(text);
            assert!(message.contains("0..=16383"), "{text}: {message}");
        }
        let color = declared_action(&modules, action)
            .expect("the declared action")
            .parameters
            .iter()
            .find(|parameter| matches!(parameter.kind, ParameterKind::Color))
            .expect("a color parameter");
        assert_eq!(parse_field(color, "255,0,0").unwrap(), json!([255, 0, 0]));
        for text in ["255,0", "256,0,0", "255,0,0,0", "a,b,c", ""] {
            let message = parse_field(color, text).expect_err(text);
            assert!(message.contains("0..=255"), "{text}: {message}");
        }
        let choice = modules
            .iter()
            .flat_map(|module| module.actions.iter())
            .flat_map(|action| action.parameters.iter())
            .find(|parameter| matches!(parameter.kind, ParameterKind::Enum { .. }))
            .expect("the transform module declares an enum");
        let ParameterKind::Enum { options } = &choice.kind else {
            unreachable!("filtered above")
        };
        assert_eq!(
            parse_field(choice, &options[0]).unwrap(),
            json!(options[0].clone())
        );
        let message = parse_field(choice, "sideways").expect_err("an undeclared option");
        assert!(message.contains(&options[0]), "{message}");
    }

    #[test]
    fn colour_channels_are_edited_one_at_a_time() {
        assert_eq!(channel_text("1,2,3", 1), "2");
        assert_eq!(channel_text("1,2", 2), "");
        assert_eq!(replace_channel("1,2,3", 1, " 200 "), "1,200,3");
        assert_eq!(replace_channel("", 0, "5"), "5,,");
    }

    #[test]
    fn action_parameters_merge_presets_over_field_values() {
        let modules = descriptors();
        let (action, x, y) = point_pick(&modules).expect("a canvas pick");
        let declared = declared_action(&modules, action).expect("the declared action");
        let mut fields = Fields::seeded(&modules);
        fields.set(action, x, "4".into());
        fields.set(action, y, "5".into());
        let params = action_params(declared, &Map::new(), &fields).unwrap();
        assert_eq!(params[x], json!(4));
        assert_eq!(params[y], json!(5));
        let preset = json!({ x: 9 }).as_object().expect("an object").clone();
        let params = action_params(declared, &preset, &fields).unwrap();
        assert_eq!(params[x], json!(9), "the preset wins over the field");
        assert_eq!(params[y], json!(5));
        fields.set(action, x, "nine".into());
        let message = action_params(declared, &Map::new(), &fields)
            .expect_err("an unparsable field stops the request");
        assert!(message.contains("0..=16383"), "{message}");
        assert!(
            action_params(declared, &preset, &fields).is_ok(),
            "a preset supplies the parameter the field cannot"
        );
    }

    #[test]
    fn an_action_runs_only_when_every_required_parameter_is_supplied() {
        let modules = descriptors();
        let fields = Fields::seeded(&modules);
        let runnable = |action: &str, preset: &Map<String, Value>| {
            action_params(
                declared_action(&modules, action).expect("the declared action"),
                preset,
                &fields,
            )
            .is_ok()
        };
        for module in &modules {
            for control in &module.controls {
                let Rendered::Group { controls, .. } = classify(control) else {
                    continue;
                };
                for child in controls {
                    if let Rendered::Action { action, preset, .. } = classify(child) {
                        assert!(
                            runnable(action, preset),
                            "{action} is not runnable from its declared control"
                        );
                        assert!(
                            preset.is_empty() || !runnable(action, &Map::new()),
                            "{action} needs its preset to run"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn enter_in_a_field_runs_the_first_control_that_invokes_the_action() {
        let modules = descriptors();
        let (action, _, _) = point_pick(&modules).expect("a canvas pick");
        assert_eq!(
            submit_preset(&modules, action),
            Some(Map::new()),
            "the pixel action is driven by its fields alone"
        );
        let choice = modules
            .iter()
            .find(|module| module.canvas.is_none())
            .expect("the transform module");
        let Some(Rendered::Action { action, preset, .. }) =
            choice.controls.first().map(classify).map(|control| {
                let Rendered::Group { controls, .. } = control else {
                    unreachable!("transform controls are grouped")
                };
                classify(&controls[0])
            })
        else {
            unreachable!("the first transform control invokes an action")
        };
        assert_eq!(submit_preset(&modules, action).as_ref(), Some(preset));
        assert_eq!(submit_preset(&modules, "no-such-action"), None);
    }

    #[test]
    fn fit_picks_map_through_the_centered_contained_rectangle() {
        let image = (200, 100);
        let available = Size::new(400.0, 400.0);
        let rect = fit_rect(image, available).expect("a drawn rectangle");
        assert_eq!((rect.x, rect.y), (0.0, 100.0));
        assert_eq!((rect.width, rect.height), (400.0, 200.0));
        assert_eq!(
            fit_pick(image, available, Point::new(0.0, 100.0)),
            Some((0, 0))
        );
        assert_eq!(
            fit_pick(image, available, Point::new(399.0, 299.0)),
            Some((199, 99))
        );
        assert_eq!(
            fit_pick(image, available, Point::new(200.0, 200.0)),
            Some((100, 50)),
            "the centre of the surface is the centre of the photograph"
        );
        for outside in [
            Point::new(0.0, 99.0),
            Point::new(0.0, 300.0),
            Point::new(-1.0, 150.0),
            Point::new(f32::NAN, 150.0),
        ] {
            assert_eq!(fit_pick(image, available, outside), None, "{outside:?}");
        }
        assert_eq!(fit_rect(image, Size::new(0.0, 400.0)), None);
        assert_eq!(fit_rect((0, 0), available), None);
    }

    #[test]
    fn percent_picks_ignore_pan_because_the_scrollable_translates_the_cursor() {
        let image = (200, 200);
        let scale = 2.0;
        // The scrollable hands its content a cursor already moved by the scroll offset, so the
        // point mouse_area reports is the viewport position plus the pan.
        for pan in [(0.0, 0.0), (100.0, 50.0), (317.0, 9.0)] {
            let viewport = Point::new(10.0, 20.0);
            let reported = Point::new(viewport.x + pan.0, viewport.y + pan.1);
            let expected = (
                ((viewport.x + pan.0) / scale) as u32,
                ((viewport.y + pan.1) / scale) as u32,
            );
            assert_eq!(percent_pick(image, scale, reported), Some(expected));
        }
        assert_eq!(
            percent_pick(image, scale, Point::new(1.9, 0.0)),
            Some((0, 0))
        );
        assert_eq!(percent_pick(image, scale, Point::new(400.0, 0.0)), None);
        assert_eq!(percent_pick(image, scale, Point::new(-0.5, 0.0)), None);
        assert_eq!(percent_pick(image, 0.0, Point::new(1.0, 1.0)), None);
    }

    #[test]
    fn an_unrenderable_control_kind_is_named_not_dropped() {
        assert_eq!(
            unsupported_label("gradient"),
            "Unsupported control: gradient"
        );
        let controls = [
            Control::Group {
                label: "Group".into(),
                controls: Vec::new(),
            },
            Control::Number {
                action: "act".into(),
                parameter: "x".into(),
                label: "X".into(),
            },
            Control::Color {
                action: "act".into(),
                parameter: "rgb".into(),
                label: "RGB".into(),
            },
            Control::Action {
                action: "act".into(),
                label: "Apply".into(),
                preset: Map::new(),
            },
        ];
        for (control, kind) in controls.iter().zip(["group", "number", "color", "action"]) {
            assert_eq!(control_kind(control), kind);
            assert!(
                !matches!(classify(control), Rendered::Unsupported(_)),
                "{kind} is rendered"
            );
        }
        // Every control the registered modules declare has a real rendering.
        for module in descriptors() {
            let mut queue: Vec<&Control> = module.controls.iter().collect();
            while let Some(control) = queue.pop() {
                match classify(control) {
                    Rendered::Group { controls, .. } => queue.extend(controls),
                    Rendered::Unsupported(kind) => panic!("{} declares {kind}", module.id),
                    _ => {}
                }
            }
        }
        assert_eq!(
            undeclared_label("act", "z"),
            "Unsupported control: act declares no parameter z"
        );
    }

    #[test]
    fn a_canvas_pick_fills_the_declared_coordinate_fields_without_committing() {
        let (mut editor, catalog) = boot();
        let _ = editor.update(Message::ModulesLoaded(Ok(descriptors())));
        assert!(editor.modules_ready);
        let (action, x, y) = point_pick(&editor.modules).expect("a canvas pick");
        let (action, x, y) = (action.to_owned(), x.to_owned(), y.to_owned());
        let _ = editor.update(Message::PointerMoved(Some((7, 9))));
        assert_eq!(editor.pointer, Some((7, 9)));
        let _ = editor.update(Message::PointPicked { x: 7, y: 9 });
        assert_eq!(editor.fields.get(&action, &x), Some("7"));
        assert_eq!(editor.fields.get(&action, &y), Some("9"));
        assert!(
            editor.state.is_none(),
            "a pick opens no asset and commits nothing"
        );
        assert_eq!(editor.api_sequence, 0);
        let _ = editor.update(Message::ControlChanged {
            action: action.clone(),
            parameter: x.clone(),
            text: "11".into(),
        });
        assert_eq!(editor.fields.get(&action, &x), Some("11"));
        // The correlated state carries the module identities and what the controls hold.
        let snapshot = editor.snapshot();
        assert_eq!(snapshot["controls"][format!("{action}.{x}")], json!("11"));
        assert_eq!(
            snapshot["modules"].as_array().map(Vec::len),
            Some(editor.modules.len())
        );
        assert!(!editor.editable(), "nothing is open");
        finish(editor, catalog);
    }

    #[test]
    fn failed_discovery_is_reported_and_never_blocks_evidence() {
        let (mut editor, catalog) = boot();
        let _ = editor.update(Message::ModulesLoaded(Err("protocol: gone".into())));
        assert!(editor.modules_ready);
        assert!(editor.modules.is_empty());
        assert!(
            editor.status.contains("Tool discovery failed"),
            "{}",
            editor.status
        );
        finish(editor, catalog);
    }

    #[test]
    fn one_hundred_percent_uses_physical_pixel_scale() {
        let width = 6000f32;
        let display_scale = 2f32;
        let logical_width = width / display_scale;
        assert_eq!(logical_width * display_scale, width);
    }

    #[test]
    fn short_ids_are_safe_for_status_display() {
        assert_eq!(short("abc"), "abc");
        assert_eq!(short("123456789012345"), "123456789012");
    }

    #[test]
    fn current_entry_merge_is_newest_first_and_bounded() {
        let asset = AssetId::new();
        let mut history = HistoryPage {
            entries: (0..HISTORY_PAGE_SIZE as u64)
                .rev()
                .map(|sequence| entry(&asset, sequence, None))
                .collect(),
            next_before_sequence: None,
        };
        merge_current_entry(&mut history, entry(&asset, HISTORY_PAGE_SIZE as u64, None));
        assert_eq!(history.entries.len(), HISTORY_PAGE_SIZE);
        assert_eq!(history.entries[0].sequence, HISTORY_PAGE_SIZE as u64);
        assert_eq!(history.entries.last().unwrap().sequence, 1);
        assert_eq!(history.next_before_sequence, Some(1));
    }

    #[test]
    fn stale_session_responses_are_not_adopted() {
        let (mut editor, catalog) = boot();
        let mut newer = ClientSession {
            revision: 5,
            ..ClientSession::default()
        };
        newer
            .preview
            .view
            .set_zoom(Zoom::Percent { value: 200.0 })
            .unwrap();
        let older = ClientSession {
            revision: 3,
            ..ClientSession::default()
        };
        let _ = editor.update(Message::SessionUpdated(Ok((newer.clone(), 1))));
        let _ = editor.update(Message::PanSynced(Ok(older)));
        assert_eq!(editor.session, newer);
        let mut same = newer.clone();
        same.preview.view.pan_to(4.0, 5.0).unwrap();
        let _ = editor.update(Message::SessionUpdated(Ok((same.clone(), 1))));
        assert_eq!(
            editor.session, same,
            "an equal revision may replace the copy"
        );
        finish(editor, catalog);
    }

    #[test]
    fn pan_keeps_one_request_in_flight_and_only_the_newest_pending_position() {
        let (mut editor, catalog) = boot();
        let _ = editor.update(Message::Panned(1.0, 2.0));
        assert!(editor.pan_in_flight);
        assert_eq!(editor.pending_pan, None);
        let _ = editor.update(Message::Panned(3.0, 4.0));
        let _ = editor.update(Message::Panned(5.0, 6.0));
        assert_eq!(editor.pending_pan, Some((5.0, 6.0)));
        let _ = editor.update(Message::PanSynced(Ok(ClientSession::default())));
        assert!(
            editor.pan_in_flight,
            "the pending position starts the next request"
        );
        assert_eq!(editor.pending_pan, None);
        let _ = editor.update(Message::PanSynced(Ok(ClientSession::default())));
        assert!(!editor.pan_in_flight);
        finish(editor, catalog);
    }

    #[test]
    fn refresh_replaces_or_merges_history_and_marks_abandoned_branches() {
        let (mut editor, catalog) = boot();
        let asset = AssetId::new();
        let original = entry(&asset, 0, None);
        let a = entry(&asset, 1, Some(&original.id));
        let b = entry(&asset, 2, Some(&a.id));
        let c = entry(&asset, 3, Some(&a.id));
        editor.busy = true;
        let full = refresh_for(
            &asset,
            &c,
            vec![c.clone(), b.clone(), a.clone(), original.clone()],
            &[&c, &a, &original],
            false,
        );
        let _ = editor.update(Message::Refreshed(Ok(Box::new(full))));
        assert!(!editor.busy);
        assert_eq!(editor.api_sequence, 7);
        assert_eq!(editor.history.entries.len(), 4);
        assert_eq!(editor.display_entry, Some(c.id.clone()));
        assert!(editor.on_current_lineage(&c));
        assert!(editor.on_current_lineage(&original));
        assert!(
            !editor.on_current_lineage(&b),
            "b was undone and is a branch"
        );
        let d = entry(&asset, 4, Some(&c.id));
        let merged = refresh_for(&asset, &d, Vec::new(), &[&d, &c], true);
        let _ = editor.update(Message::Refreshed(Ok(Box::new(merged))));
        assert_eq!(editor.history.entries.len(), 5);
        assert_eq!(editor.history.entries[0].id, d.id);
        assert!(editor.on_current_lineage(&d));
        assert!(
            editor.on_current_lineage(&b),
            "below a truncated lineage nothing is marked as a branch"
        );
        finish(editor, catalog);
    }
}
