use crate::{
    Config,
    crop_canvas::{CropCanvas, Mode, Part, View},
    crop_draft::{AspectPreset, CropDraft, Handle, Modifiers as DraftModifiers},
    diagnostics::Diagnostics,
    paths::Paths,
};
use iced::{
    ContentFit, Element, Length, Point, Rectangle, Size, Subscription, Task,
    widget::{
        button, canvas, column, container, image, mouse_area, operation, responsive, row,
        scrollable, stack, text, text_input,
    },
};
use iced_runtime::image as image_memory;
use lightwell_core::{
    ActionDescriptor, ApiRequest, AssetId, Availability, CanvasInteraction, ClientId,
    ClientSession, Control, CropPayload, CropStage, EditorState, EffectStage, EntryId, ErrorKind,
    EventsResult, HistoryEntry, HistoryPage, HistorySelection, LayerId, Lineage, LocalServer,
    MAX_ANGLE, MIN_ANGLE, ModuleDescriptor, Mutation, OwnerHandle, ParameterDescriptor,
    ParameterKind, PreviewJob, PreviewQueue, Version, Zoom,
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
/// The scrollable around the photo, so a Space drag can scroll it while drafting a crop.
const SURFACE_ID: &str = "lightwell.surface";
/// How far one nudge button moves the straightening angle, in degrees.
const ANGLE_STEP: f64 = 0.5;
/// Ratio presets per row in the sidebar.
const PRESETS_PER_ROW: usize = 3;

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
pub(crate) struct Refresh {
    state: EditorState,
    history: Option<HistoryPage>,
    versions: Vec<Version>,
    lineage: Lineage,
    job: PreviewJob,
    session: ClientSession,
    sequence: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct PreviewPayload {
    job: PreviewJob,
    session: ClientSession,
    sequence: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct Upload {
    generation: u64,
    width: u32,
    height: u32,
    entry_id: EntryId,
    snapshot_id: String,
    source_fingerprint: String,
    started: Instant,
}

#[derive(Clone, Debug)]
pub(crate) enum SyncResult {
    Unchanged { sequence: u64 },
    Changed(Box<Refresh>),
}

/// What a crop draft is waiting for its truncated preview to tell it: the layer it edits and the
/// payload it starts from are known from the stack, but the input stage is whatever that preview
/// renders, so the draft opens when its pixels arrive.
#[derive(Clone, Debug)]
struct PendingDraft {
    layer: Option<LayerId>,
    layer_index: usize,
    payload: Option<CropPayload>,
    base_revision: u64,
    /// A reapply rebases the existing draft instead of opening a new one.
    reapply: bool,
}

/// One pointer step of a crop gesture, already mapped to box pixels by the canvas.
#[derive(Clone, Copy, Debug)]
pub(crate) enum CropPointer {
    Begin { handle: Handle, x: f64, y: f64 },
    Drag { x: f64, y: f64, option: bool },
    End,
}

/// Every crop draft change is one message, so a script can drive the whole editor through the
/// update function without simulating a pointer.
#[derive(Clone, Debug)]
pub(crate) enum CropMessage {
    /// Open a draft on the current stack.
    Start,
    /// Re-read the current stack and rebase the conflicted draft onto it.
    Reapply,
    /// The truncated preview job for a start or a reapply.
    PreviewReady(Result<Box<PreviewJob>, String>),
    Pointer(CropPointer),
    AngleText(String),
    SubmitAngle,
    NudgeAngle(f64),
    /// The index of one generated ratio preset.
    Preset(usize),
    CustomWidth(String),
    CustomHeight(String),
    Swap,
    Lock,
    /// The Straighten guide toggle: a drag on the image draws a levelling line instead.
    Guide(bool),
    /// Option (Alt) is held, so a handle scales uniformly about the centre.
    Option(bool),
    /// Space is held, so a drag pans instead of touching the draft.
    Space(bool),
    /// A Space drag asked for this many logical pixels of scroll.
    Pan {
        dx: f32,
        dy: f32,
    },
    Apply,
    Cancel,
}

#[derive(Clone, Debug)]
pub(crate) enum Message {
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
    /// The truncated preview of a crop layer's input stage reached the GPU.
    DraftUploaded(
        Upload,
        Result<image_memory::Allocation, image_memory::Error>,
    ),
    Crop(CropMessage),
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
    /// The transient crop draft. It is session state, never authoritative: only Apply commits.
    crop: Option<CropDraft>,
    /// What a started or reapplied draft still needs from its truncated preview.
    crop_pending: Option<PendingDraft>,
    /// The crop layer's input stage on the GPU: one extra texture, bounded like the main preview
    /// and dropped as soon as the draft ends.
    draft_photo: Option<image_memory::Allocation>,
    /// The preview generation that belongs to the draft rather than to the displayed state.
    draft_generation: Option<u64>,
    /// This desktop's own Apply is in flight, so the revision it produces is not a conflict.
    crop_applying: Option<String>,
    crop_angle: String,
    /// The two extents the `custom` ratio preset reads.
    crop_custom: (String, String),
    crop_guide: bool,
    crop_option: bool,
    crop_space: bool,
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
            crop: None,
            crop_pending: None,
            draft_photo: None,
            draft_generation: None,
            crop_applying: None,
            crop_angle: "0".into(),
            crop_custom: ("5".into(), "4".into()),
            crop_guide: false,
            crop_option: false,
            crop_space: false,
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
        json!({"run_id":self.run_id,"mode":if self.evidence.is_some() {"evidence"} else {"editor"},"orientation":self.activity.orientation,"phase":self.activity.phase,"requested_generation":self.activity.requested,"displayed_generation":self.activity.displayed,"source_dimensions":self.activity.source_dimensions,"preview_dimensions":self.activity.preview_dimensions,"backend":self.activity.backend,"status":self.status,"error_code":self.activity.error_code,"modules":module_summary(&self.modules),"controls":self.fields.summary(),"crop":self.crop_summary()})
    }

    /// The crop draft as a captured frame reports it, so a rendered frame correlates with the
    /// rectangle, angle and output size that produced it.
    fn crop_summary(&self) -> Value {
        match &self.crop {
            Some(draft) => {
                let mut summary = draft.summary();
                if let Some(object) = summary.as_object_mut() {
                    object.insert("guide".into(), Value::from(self.crop_guide));
                    object.insert("option".into(), Value::from(self.crop_option));
                    object.insert("space".into(), Value::from(self.crop_space));
                    object.insert(
                        "paused".into(),
                        Value::from(!self.session.preview.can_edit()),
                    );
                    object.insert(
                        "input_stage_loaded".into(),
                        Value::from(self.draft_photo.is_some()),
                    );
                }
                summary
            }
            None => json!({"drafting":false,"pending":self.crop_pending.is_some()}),
        }
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
                        // A failed Apply keeps the draft; a stale revision makes it conflicted so
                        // the user chooses Discard or Reapply rather than losing the composition.
                        if self.crop_applying.take().is_some() {
                            let conflict = error.starts_with(ErrorKind::Conflict.code());
                            if conflict && let Some(draft) = &mut self.crop {
                                draft.mark_conflicted();
                            }
                            if conflict {
                                self.crop_changed("crop_draft_conflicted");
                            }
                        }
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
                    // The draft's truncated preview shares the queue; its generation says which
                    // texture the pixels belong to.
                    let for_draft = Some(result.generation) == self.draft_generation;
                    if !for_draft && result.generation != self.preview_generation {
                        return Task::none();
                    }
                    match result.result {
                        Ok(raster) => {
                            self.uploading = true;
                            self.status = "Preparing pixels for display…".into();
                            if self.activity.pending && !for_draft {
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
                            return image_memory::allocate(handle).map(move |result| {
                                if for_draft {
                                    Message::DraftUploaded(upload.clone(), result)
                                } else {
                                    Message::Uploaded(upload.clone(), result)
                                }
                            });
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
            Message::DraftUploaded(upload, result) => {
                self.uploading = false;
                if Some(upload.generation) != self.draft_generation {
                    return Task::none();
                }
                match result {
                    Ok(allocation) => {
                        self.draft_photo = Some(allocation);
                        self.open_draft(CropStage {
                            width: upload.width,
                            height: upload.height,
                            angle: 0.0,
                        });
                    }
                    Err(_) => {
                        self.crop_pending = None;
                        self.draft_generation = None;
                        self.status = "Could not upload the crop's input stage".into();
                    }
                }
            }
            Message::Crop(message) => return self.crop_update(message),
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
        let revision = refresh.state.revision;
        let entry = refresh.state.current_entry.id.clone();
        self.state = Some(refresh.state);
        self.display_entry = Some(refresh.job.entry.id.clone());
        self.preview_generation = self.preview_queue.request(refresh.job);
        self.status = "Rendering selected history state…".into();
        self.settle_draft(revision, &entry);
    }

    /// A new authoritative revision arrived while a draft was open. The draft's own Apply ends it;
    /// anything else, including this desktop's undo, redo and restore, marks it conflicted and keeps
    /// it, because no history operation discards a draft implicitly.
    fn settle_draft(&mut self, revision: u64, entry: &EntryId) {
        let Some((base, conflicted, summary)) = self
            .crop
            .as_ref()
            .map(|draft| (draft.base_revision, draft.conflicted, draft.summary()))
        else {
            return;
        };
        if let Some(request_id) = self.crop_applying.take() {
            self.end_draft();
            self.event(
                "crop_draft_applied",
                json!({"request_id":request_id,"entry_id":entry.as_str(),"revision":revision,"draft":summary}),
            );
            self.status = format!("Crop applied · entry {}", short(entry.as_str()));
            return;
        }
        if base == revision || conflicted {
            return;
        }
        if let Some(draft) = &mut self.crop {
            draft.mark_conflicted();
        }
        self.crop_changed("crop_draft_conflicted");
        self.status = "Changed elsewhere: discard the crop draft or reapply it".into();
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

    // ---- crop draft ----------------------------------------------------------------------------

    /// Every crop draft change goes through here, so the API-equivalent path and the pointer path
    /// are the same code.
    fn crop_update(&mut self, message: CropMessage) -> Task<Message> {
        match message {
            CropMessage::Option(option) => self.crop_option = option,
            CropMessage::Space(space) => self.crop_space = space,
            CropMessage::Guide(guide) => self.crop_guide = guide && self.crop.is_some(),
            CropMessage::Start => return self.crop_start(false),
            CropMessage::Reapply => return self.crop_start(true),
            CropMessage::PreviewReady(result) => match result {
                Ok(job) => {
                    self.draft_generation = Some(self.preview_queue.request(*job));
                    self.status = "Rendering the crop's input stage…".into();
                }
                Err(error) => {
                    self.crop_pending = None;
                    self.status = error;
                }
            },
            CropMessage::Pointer(pointer) => {
                let Some(draft) = &mut self.crop else {
                    return Task::none();
                };
                match pointer {
                    CropPointer::Begin { handle, x, y } => draft.begin(handle, (x, y)),
                    // A pointer move never calls an API and never logs: only the end of the gesture
                    // is one draft change.
                    CropPointer::Drag { x, y, option } => {
                        draft.drag((x, y), DraftModifiers { option })
                    }
                    CropPointer::End => {
                        draft.end();
                        self.crop_changed("crop_draft_changed");
                    }
                }
            }
            CropMessage::AngleText(text) => self.crop_angle = text,
            CropMessage::SubmitAngle => {
                let Ok(value) = self.crop_angle.trim().parse::<f64>() else {
                    self.status =
                        format!("Angle must be a number from {MIN_ANGLE} to {MAX_ANGLE} degrees");
                    return Task::none();
                };
                if let Some(draft) = &mut self.crop {
                    draft.set_angle(value);
                }
                self.crop_changed("crop_draft_changed");
            }
            CropMessage::NudgeAngle(step) => {
                if let Some(draft) = &mut self.crop {
                    draft.nudge_angle(step);
                }
                self.crop_changed("crop_draft_changed");
            }
            CropMessage::Preset(index) => {
                let Some(preset) = crop_frame(&self.modules)
                    .map(|frame| frame.presets())
                    .and_then(|presets| presets.get(index).cloned())
                else {
                    return Task::none();
                };
                let custom = self.custom_ratio();
                if let Some(draft) = &mut self.crop {
                    draft.set_preset(&preset, custom);
                }
                self.crop_changed("crop_draft_changed");
            }
            CropMessage::CustomWidth(text) => self.crop_custom.0 = text,
            CropMessage::CustomHeight(text) => self.crop_custom.1 = text,
            CropMessage::Swap => {
                if let Some(draft) = &mut self.crop {
                    draft.swap();
                }
                self.crop_changed("crop_draft_changed");
            }
            CropMessage::Lock => {
                if let Some(draft) = &mut self.crop {
                    draft.lock_toggle();
                }
                self.crop_changed("crop_draft_changed");
            }
            CropMessage::Pan { dx, dy } => {
                if dx == 0.0 && dy == 0.0 {
                    return Task::none();
                }
                return operation::scroll_by(
                    SURFACE_ID,
                    operation::AbsoluteOffset { x: dx, y: dy },
                );
            }
            CropMessage::Apply => {
                if self.busy || !self.session.preview.can_edit() {
                    return Task::none();
                }
                match self.crop_request() {
                    None => {}
                    Some(Err(message)) => self.status = message,
                    Some(Ok((method, request, request_id))) => {
                        self.crop_applying = Some(request_id);
                        return self.command(method, request);
                    }
                }
            }
            CropMessage::Cancel => {
                let Some(summary) = self.crop.as_ref().map(CropDraft::summary) else {
                    return Task::none();
                };
                self.end_draft();
                self.event("crop_draft_discarded", summary);
                self.status = "Crop draft discarded".into();
            }
        }
        Task::none()
    }

    /// Open a draft, or re-read the stack for a reapply. The layer identity, its stored payload and
    /// the preview truncation come from the current stack; the input stage comes from that preview.
    fn crop_start(&mut self, reapply: bool) -> Task<Message> {
        if self.busy || !self.session.preview.can_edit() || reapply != self.crop.is_some() {
            return Task::none();
        }
        let Some(effect) = crop_frame(&self.modules)
            .and_then(|frame| frame.effect().map(str::to_owned))
            .filter(|_| self.state.is_some())
        else {
            return Task::none();
        };
        let state = self.state.as_ref().expect("filtered above");
        let layers = &state.current_entry.snapshot.recipe.layers;
        let found = layers.iter().position(|layer| layer.effect_id == effect);
        let pending = PendingDraft {
            layer: found.map(|index| layers[index].id.clone()),
            layer_index: found.unwrap_or(layers.len()),
            payload: found
                .and_then(|index| serde_json::from_value(layers[index].payload.clone()).ok()),
            base_revision: state.revision,
            reapply,
        };
        let asset = state.asset.id.clone();
        let layer_count = pending.layer_index;
        self.crop_pending = Some(pending);
        self.status = "Preparing the crop's input stage…".into();
        crop_preview_task(self.owner.clone(), asset, layer_count)
    }

    /// The truncated preview arrived, so the crop layer's input stage is known: open or rebase the
    /// draft against it.
    fn open_draft(&mut self, input: CropStage) {
        let Some(pending) = self.crop_pending.take() else {
            return;
        };
        if pending.reapply {
            match &mut self.crop {
                Some(draft) => draft.rebase(
                    input,
                    pending.base_revision,
                    pending.layer,
                    pending.layer_index,
                ),
                None => return,
            }
        } else {
            self.crop = match (pending.layer, pending.payload) {
                (Some(layer), Some(payload)) => Some(CropDraft::from_layer(
                    input,
                    payload,
                    layer,
                    pending.layer_index,
                    pending.base_revision,
                )),
                // An unreadable payload is never silently replaced by a neutral crop: the stored
                // layer stays exactly as it is and the draft does not open.
                (Some(_), None) => {
                    self.draft_photo = None;
                    self.draft_generation = None;
                    self.status =
                        "The existing crop layer's payload cannot be read; no draft was opened"
                            .into();
                    return;
                }
                (None, _) => Some(CropDraft::neutral(
                    input,
                    pending.base_revision,
                    pending.layer_index,
                )),
            };
        }
        self.crop_changed(if pending.reapply {
            "crop_draft_changed"
        } else {
            "crop_draft_started"
        });
        self.status = format!(
            "Crop draft on the layer's {} × {} input stage",
            input.width, input.height
        );
    }

    /// One draft change reached its end: the angle field follows the draft and the new state is
    /// logged. Pointer moves inside a gesture do not come through here.
    fn crop_changed(&mut self, event: &'static str) {
        let Some((angle, summary)) = self
            .crop
            .as_ref()
            .map(|draft| (number_text(draft.stage.angle), draft.summary()))
        else {
            return;
        };
        self.crop_angle = angle;
        self.event(event, summary);
    }

    /// Drop the draft and the extra texture it displayed.
    fn end_draft(&mut self) {
        self.crop = None;
        self.crop_pending = None;
        self.draft_photo = None;
        self.draft_generation = None;
        self.crop_applying = None;
        self.crop_guide = false;
    }

    /// The `custom` preset's two extents as typed, or `None` when either is not a positive number.
    fn custom_ratio(&self) -> Option<(f64, f64)> {
        let width: f64 = self.crop_custom.0.trim().parse().ok()?;
        let height: f64 = self.crop_custom.1.trim().parse().ok()?;
        (width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0)
            .then_some((width, height))
    }

    /// The request Apply would send, built from the canvas descriptor's own parameter names, or the
    /// validation message that stops it. `None` means there is nothing to apply.
    fn crop_request(&self) -> Option<Result<(String, Value, String), String>> {
        let frame = crop_frame(&self.modules)?;
        let draft = self.crop.as_ref()?;
        let state = self.state.as_ref()?;
        if draft.conflicted {
            return None;
        }
        if let Err(error) = draft.output() {
            return Some(Err(error.to_string()));
        }
        let mutation = mutation(draft.base_revision);
        let request_id = mutation.request_id.clone();
        let mut request = json!({"asset_id":state.asset.id,"mutation":mutation});
        request
            .as_object_mut()
            .expect("the envelope is an object")
            .extend(frame.params(&draft.payload()));
        Some(Ok((format!("edit.{}", frame.action), request, request_id)))
    }

    /// The crop draft is displayed instead of the plain preview only while its own input stage is on
    /// the GPU and the session shows the current state.
    fn drafting(&self) -> bool {
        self.crop.is_some() && self.draft_photo.is_some() && self.session.preview.can_edit()
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
            // A declared crop frame is a host interaction, not a control: the host renders its draft
            // panel here and the module's own controls, Reset crop included, still come below.
            if let Some(frame) =
                crop_frame(&self.modules).filter(|frame| frame.module.id == module.id)
            {
                panel = panel.push(self.crop_section(&frame, enabled));
            }
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

    /// The crop frame over the layer's own input stage, at Fit or at a percentage zoom. The canvas
    /// draws nothing authoritative: it borrows the draft and publishes messages.
    fn crop_surface<'a>(
        &'a self,
        draft: &'a CropDraft,
        allocation: &'a image_memory::Allocation,
    ) -> Element<'a, Message> {
        let handle = allocation.handle().clone();
        let box_size = draft.stage.bounding_box();
        let mode = if self.crop_space {
            Mode::Pan
        } else if self.crop_guide {
            Mode::Guide
        } else {
            Mode::Frame
        };
        let option = self.crop_option;
        // Two stacked canvases: iced paints every image of one layer over every mesh of that layer,
        // so the frame, thirds, handles and guide need the layer the stack gives its second child.
        let parts = move |handle: image::Handle, view: View, width: Length, height: Length| {
            stack([Part::Photo, Part::Overlay].map(|part| {
                canvas(CropCanvas::new(
                    draft,
                    handle.clone(),
                    view,
                    mode,
                    option,
                    part,
                ))
                .width(width)
                .height(height)
                .into()
            }))
        };
        match self.session.preview.view.zoom {
            Zoom::Fit => responsive(move |available| match View::fit(box_size, available) {
                Some(view) => parts(handle.clone(), view, Length::Fill, Length::Fill).into(),
                None => container(text("The surface is too small to draw the crop").size(12))
                    .center(Length::Fill)
                    .into(),
            })
            .into(),
            Zoom::Percent { value } => {
                let Some(view) = View::percent(value, self.scale_factor) else {
                    return container(text("Zoom is out of range").size(12))
                        .center(Length::Fill)
                        .into();
                };
                let frame = parts(
                    handle,
                    view,
                    Length::Fixed(box_size.0 as f32 * view.scale),
                    Length::Fixed(box_size.1 as f32 * view.scale),
                );
                scrollable(container(frame).center(Length::Shrink))
                    .id(SURFACE_ID)
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
        }
    }

    /// The crop draft's own controls, rendered by the host for a declared crop-frame interaction.
    /// The module's generic controls, including Reset crop, still come from its descriptor.
    fn crop_section<'a>(&'a self, frame: &CropFrame<'a>, enabled: bool) -> Element<'a, Message> {
        let mut panel = column![text("Crop").size(18)].spacing(8);
        let Some(draft) = &self.crop else {
            panel = panel.push(
                button("Crop").on_press_maybe(
                    (enabled && self.crop_pending.is_none())
                        .then_some(Message::Crop(CropMessage::Start)),
                ),
            );
            if self.crop_pending.is_some() {
                panel = panel.push(text("Preparing the crop's input stage…").size(12));
            }
            return panel.into();
        };
        if draft.conflicted {
            panel = panel.push(text("Changed elsewhere: Discard or Reapply").size(12));
            panel = panel.push(
                row![
                    button("Discard").on_press(Message::Crop(CropMessage::Cancel)),
                    button("Reapply").on_press_maybe(
                        (!self.busy).then_some(Message::Crop(CropMessage::Reapply))
                    ),
                ]
                .spacing(6),
            );
        }
        if !self.session.preview.can_edit() {
            panel = panel
                .push(text("Draft paused during history preview · Return to current").size(12));
        }
        // Ratio presets, generated from the fit action's declared aspect options.
        let presets = frame.presets();
        for chunk in presets.chunks(PRESETS_PER_ROW) {
            let mut buttons = row![].spacing(6);
            for preset in chunk {
                let index = presets
                    .iter()
                    .position(|candidate| candidate.option == preset.option)
                    .unwrap_or_default();
                let chosen = draft.preset == preset.option;
                let label = if chosen {
                    format!("● {}", preset.label())
                } else {
                    preset.label()
                };
                buttons =
                    buttons.push(button(text(label).size(12)).on_press_maybe(
                        enabled.then_some(Message::Crop(CropMessage::Preset(index))),
                    ));
            }
            panel = panel.push(buttons);
        }
        panel = panel.push(
            row![
                text("Custom").size(12).width(56),
                text_input("W", &self.crop_custom.0)
                    .id(field_id(frame.fit_action, "custom-width", None))
                    .on_input(|text| Message::Crop(CropMessage::CustomWidth(text)))
                    .width(48),
                text_input("H", &self.crop_custom.1)
                    .id(field_id(frame.fit_action, "custom-height", None))
                    .on_input(|text| Message::Crop(CropMessage::CustomHeight(text)))
                    .width(48),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        );
        let lock = match draft.aspect.ratio() {
            Some(_) => "Unlock ratio",
            None => "Lock ratio",
        };
        panel = panel.push(
            row![
                button(text(lock).size(12))
                    .on_press_maybe(enabled.then_some(Message::Crop(CropMessage::Lock))),
                button(text("Swap").size(12)).on_press_maybe(
                    (enabled && draft.aspect.ratio().is_some())
                        .then_some(Message::Crop(CropMessage::Swap))
                ),
            ]
            .spacing(6),
        );
        panel = panel.push(
            row![
                text("Angle (deg)").size(12).width(78),
                text_input("0", &self.crop_angle)
                    .id(field_id(frame.action, frame.angle, None))
                    .on_input(|text| Message::Crop(CropMessage::AngleText(text)))
                    .on_submit(Message::Crop(CropMessage::SubmitAngle))
                    .width(60),
                button(text(format!("−{ANGLE_STEP}°")).size(12)).on_press_maybe(
                    enabled.then_some(Message::Crop(CropMessage::NudgeAngle(-ANGLE_STEP)))
                ),
                button(text(format!("+{ANGLE_STEP}°")).size(12)).on_press_maybe(
                    enabled.then_some(Message::Crop(CropMessage::NudgeAngle(ANGLE_STEP)))
                ),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        );
        let guide = if self.crop_guide {
            "Straighten guide: on"
        } else {
            "Straighten guide: off"
        };
        panel = panel.push(
            button(text(guide).size(12))
                .on_press(Message::Crop(CropMessage::Guide(!self.crop_guide))),
        );
        panel = panel.push(
            row![
                button("Apply").on_press_maybe(
                    (enabled && !draft.conflicted).then_some(Message::Crop(CropMessage::Apply))
                ),
                button("Cancel").on_press(Message::Crop(CropMessage::Cancel)),
            ]
            .spacing(6),
        );
        // The draft's own numbers, so what is on screen is observable without a debugger.
        let payload = draft.payload();
        let output = match draft.output() {
            Ok(rect) => format!(
                "{} × {} px at ({}, {})",
                rect.width, rect.height, rect.x, rect.y
            ),
            Err(error) => error.detail.clone(),
        };
        panel = panel.push(
            text(format!(
                "Input stage {} × {} · box {:.0} × {:.0}\nRect {:.0}, {:.0}, {:.0} × {:.0} box px\nOutput {output}\nPayload angle {} x {:.6} y {:.6} w {:.6} h {:.6}",
                draft.stage.width,
                draft.stage.height,
                draft.stage.bounding_box().0,
                draft.stage.bounding_box().1,
                draft.rect.x,
                draft.rect.y,
                draft.rect.width,
                draft.rect.height,
                number_text(payload.angle),
                payload.x,
                payload.y,
                payload.width,
                payload.height,
            ))
            .size(11),
        );
        panel = panel.push(
            text("Drag handles to resize, inside to move, Option for one scale about the centre, Space to pan. Enter applies, Escape cancels.")
                .size(11),
        );
        panel.into()
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
        // While a draft has its own input stage on the GPU the crop frame replaces the plain image;
        // during a history preview the historical preview shows and the draft is only paused.
        let drafted: Option<Element<'_, Message>> =
            match (self.drafting(), &self.crop, &self.draft_photo) {
                (true, Some(draft), Some(allocation)) => Some(self.crop_surface(draft, allocation)),
                _ => None,
            };
        let plain: Element<'_, Message> = match (&self.photo, self.dimensions) {
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
                        .id(SURFACE_ID)
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
        let surface = drafted.unwrap_or(plain);

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
        // A draft adds one keyboard listener and no timer: Enter and Escape act only on keys no text
        // input consumed, and the modifier state the canvas reads lives in the app.
        if self.crop.is_some() {
            subscriptions.push(iced::event::listen_with(|event, status, _| {
                use iced::keyboard::{Event as Keys, key::Named};
                let iced::Event::Keyboard(event) = event else {
                    return None;
                };
                match event {
                    Keys::ModifiersChanged(modifiers) => {
                        Some(Message::Crop(CropMessage::Option(modifiers.alt())))
                    }
                    Keys::KeyReleased {
                        key: iced::keyboard::Key::Named(Named::Space),
                        ..
                    } => Some(Message::Crop(CropMessage::Space(false))),
                    Keys::KeyPressed { key, modifiers, .. }
                        if status == iced::event::Status::Ignored && !modifiers.command() =>
                    {
                        match key {
                            iced::keyboard::Key::Named(Named::Space) => {
                                Some(Message::Crop(CropMessage::Space(true)))
                            }
                            iced::keyboard::Key::Named(Named::Enter) => {
                                Some(Message::Crop(CropMessage::Apply))
                            }
                            iced::keyboard::Key::Named(Named::Escape) => {
                                Some(Message::Crop(CropMessage::Cancel))
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                }
            }));
        }
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
        ParameterKind::Number { min, .. } => number_text(
            parameter
                .default
                .as_ref()
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite())
                .unwrap_or(*min),
        ),
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

fn number_range_message(name: &str, min: f64, max: f64) -> String {
    format!(
        "{name} must be a number from {} to {}",
        number_text(min),
        number_text(max)
    )
}

/// A number as a field would hold it: `0`, `-3.5`, no trailing zeros or exponent noise.
fn number_text(value: f64) -> String {
    format!("{value}")
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
        ParameterKind::Number { min, max } => text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && (min..=max).contains(&value))
            .map(Value::from)
            .ok_or_else(|| number_range_message(name, *min, *max)),
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
/// A crop frame is a different adapter and is ignored here rather than treated as a pick.
fn point_pick(modules: &[ModuleDescriptor]) -> Option<(&str, &str, &str)> {
    modules.iter().find_map(|module| match &module.canvas {
        Some(CanvasInteraction::PointPick { action, x, y }) if module.is_available() => {
            Some((action.as_str(), x.as_str(), y.as_str()))
        }
        Some(CanvasInteraction::PointPick { .. })
        | Some(CanvasInteraction::CropFrame { .. })
        | None => None,
    })
}

/// One declared crop-frame interaction: the action Apply calls, the parameter names it fills, and
/// the fit action whose `aspect` enum generates the ratio presets. The desktop reads every name from
/// here, so it knows no tool by name.
struct CropFrame<'a> {
    module: &'a ModuleDescriptor,
    action: &'a str,
    angle: &'a str,
    x: &'a str,
    y: &'a str,
    width: &'a str,
    height: &'a str,
    fit_action: &'a str,
    aspect: &'a str,
}

impl CropFrame<'_> {
    /// The durable effect identity of the crop layer: the module's geometry effect.
    fn effect(&self) -> Option<&str> {
        self.module
            .effects
            .iter()
            .find(|effect| effect.stage == EffectStage::Geometry)
            .map(|effect| effect.id.as_str())
    }

    /// The ratio presets, generated from the fit action's declared `aspect` options.
    fn presets(&self) -> Vec<AspectPreset> {
        match self
            .module
            .action(self.fit_action)
            .and_then(|action| action.parameter(self.aspect))
            .map(|parameter| &parameter.kind)
        {
            Some(ParameterKind::Enum { options }) => crate::crop_draft::aspect_presets(options),
            _ => Vec::new(),
        }
    }

    /// The payload as request fields under the declared parameter names.
    fn params(&self, payload: &CropPayload) -> Map<String, Value> {
        [
            (self.angle, payload.angle),
            (self.x, payload.x),
            (self.y, payload.y),
            (self.width, payload.width),
            (self.height, payload.height),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), Value::from(value)))
        .collect()
    }
}

/// The first available module that declares a crop frame.
fn crop_frame(modules: &[ModuleDescriptor]) -> Option<CropFrame<'_>> {
    modules.iter().find_map(|module| match &module.canvas {
        Some(CanvasInteraction::CropFrame {
            action,
            angle,
            x,
            y,
            width,
            height,
            fit_action,
            aspect,
        }) if module.is_available() => Some(CropFrame {
            module,
            action,
            angle,
            x,
            y,
            width,
            height,
            fit_action,
            aspect,
        }),
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
        .preview_job(asset_id, selected, None)
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
                .preview_job(asset_id, entry_id, None)
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

/// The crop draft's only preview job: the stack truncated to the layers before the crop layer, which
/// is exactly that layer's input stage. Starting a draft and reapplying it are the only two requests.
fn crop_preview_task(owner: OwnerHandle, asset_id: AssetId, layer_count: usize) -> Task<Message> {
    Task::perform(
        async move {
            owner
                .preview_job(asset_id, None, Some(layer_count))
                .map_err(|error| error.to_string())
        },
        |result| Message::Crop(CropMessage::PreviewReady(result.map(Box::new))),
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

    /// The crop module's descriptor as the desktop would fetch it through `module.list`. It is built
    /// here rather than taken from the registry so these tests do not depend on the module being
    /// linked: the desktop drives everything from the descriptor and knows no tool by name.
    fn crop_descriptor() -> ModuleDescriptor {
        let number = |name: &str, min: f64, max: f64, required: bool, default: Option<Value>| {
            ParameterDescriptor {
                name: name.into(),
                kind: ParameterKind::Number { min, max },
                required,
                default,
                unit: None,
                notes: "test".into(),
            }
        };
        let rectangle = ["x", "y", "width", "height"]
            .map(|name| number(name, 0.0, 1.0, true, None))
            .to_vec();
        let mut crop = vec![number(
            "angle",
            MIN_ANGLE,
            MAX_ANGLE,
            false,
            Some(json!(0.0)),
        )];
        crop.extend(rectangle.clone());
        let mut fit = vec![
            ParameterDescriptor {
                name: "aspect".into(),
                kind: ParameterKind::Enum {
                    options: CROP_ASPECTS.iter().map(|option| (*option).into()).collect(),
                },
                required: false,
                default: Some(json!("free")),
                unit: None,
                notes: "test".into(),
            },
            number("aspect-width", 1.0, 10000.0, false, None),
            number("aspect-height", 1.0, 10000.0, false, None),
            number("angle", MIN_ANGLE, MAX_ANGLE, false, Some(json!(0.0))),
        ];
        fit.push(number("center-x", 0.0, 1.0, false, None));
        fit.push(number("center-y", 0.0, 1.0, false, None));
        ModuleDescriptor {
            id: "lightwell.crop".into(),
            title: "Crop".into(),
            effects: vec![lightwell_core::EffectDescriptor {
                id: CROP_EFFECT.into(),
                format: 1,
                stage: EffectStage::Geometry,
            }],
            actions: vec![
                ActionDescriptor {
                    id: "crop".into(),
                    title: "Crop".into(),
                    notes: "test".into(),
                    parameters: crop,
                },
                ActionDescriptor {
                    id: "crop-fit".into(),
                    title: "Fit crop".into(),
                    notes: "test".into(),
                    parameters: fit,
                },
                ActionDescriptor {
                    id: "crop-reset".into(),
                    title: "Reset crop".into(),
                    notes: "test".into(),
                    parameters: Vec::new(),
                },
            ],
            controls: vec![Control::Group {
                label: "Crop".into(),
                controls: vec![Control::Action {
                    action: "crop-reset".into(),
                    label: "Reset crop".into(),
                    preset: Map::new(),
                }],
            }],
            canvas: Some(CanvasInteraction::CropFrame {
                action: "crop".into(),
                angle: "angle".into(),
                x: "x".into(),
                y: "y".into(),
                width: "width".into(),
                height: "height".into(),
                fit_action: "crop-fit".into(),
                aspect: "aspect".into(),
            }),
            availability: Availability::Available,
        }
    }

    const CROP_EFFECT: &str = "lightwell.geometry.crop";
    const CROP_ASPECTS: [&str; 7] = ["free", "original", "1:1", "3:2", "4:3", "16:9", "custom"];

    /// One layer of the crop module's effect carrying that payload.
    fn crop_layer(payload: CropPayload) -> lightwell_core::Layer {
        lightwell_core::Layer {
            id: LayerId::new(),
            effect_id: CROP_EFFECT.into(),
            effect_format: 1,
            payload: serde_json::to_value(payload).expect("a serializable payload"),
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
                layer_count: None,
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
        assert_eq!(
            seed_text(&number_parameter(None)),
            "-45",
            "a number without a default seeds at its min"
        );
        assert_eq!(
            seed_text(&number_parameter(Some(json!(0)))),
            "0",
            "a whole number default seeds without trailing noise"
        );
        assert_eq!(seed_text(&number_parameter(Some(json!(-3.5)))), "-3.5");
    }

    /// A declared number parameter: the kind the crop module's angle and rectangle use.
    fn number_parameter(default: Option<Value>) -> ParameterDescriptor {
        ParameterDescriptor {
            name: "angle".into(),
            kind: ParameterKind::Number {
                min: -45.0,
                max: 45.0,
            },
            required: true,
            default,
            unit: Some("deg".into()),
            notes: "test".into(),
        }
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
        let angle = number_parameter(None);
        for (text, expected) in [
            (" -3.5 ", json!(-3.5)),
            ("0", json!(0.0)),
            ("45", json!(45.0)),
        ] {
            assert_eq!(parse_field(&angle, text).unwrap(), expected, "{text}");
        }
        for text in ["", "45.1", "-45.1", "three", "1e400", "nan", "inf"] {
            let message = parse_field(&angle, text).expect_err(text);
            assert_eq!(
                message, "angle must be a number from -45 to 45",
                "{text}: {message}"
            );
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

    /// An editor with the crop module discovered and one asset open at that revision, whose stack is
    /// those layers.
    fn opened(
        layers: Vec<lightwell_core::Layer>,
        revision: u64,
    ) -> (Editor, PathBuf, AssetId, EntryId) {
        let (mut editor, catalog) = boot();
        let _ = editor.update(Message::ModulesLoaded(Ok(vec![crop_descriptor()])));
        let asset = AssetId::new();
        let mut current = entry(&asset, revision, None);
        for layer in layers {
            current.snapshot = current.snapshot.append(layer).expect("a valid stack");
        }
        let entry_id = current.id.clone();
        let refresh = refresh_for(&asset, &current, vec![current.clone()], &[&current], false);
        let _ = editor.update(Message::Refreshed(Ok(Box::new(refresh))));
        assert!(editor.editable(), "{}", editor.status);
        (editor, catalog, asset, entry_id)
    }

    #[test]
    fn the_crop_frame_and_its_presets_come_from_the_declared_descriptor() {
        let modules = vec![crop_descriptor()];
        let frame = crop_frame(&modules).expect("a declared crop frame");
        assert_eq!(frame.action, "crop");
        assert_eq!(frame.fit_action, "crop-fit");
        assert_eq!(frame.effect(), Some(CROP_EFFECT));
        let presets = frame.presets();
        assert_eq!(
            presets
                .iter()
                .map(|preset| preset.option.as_str())
                .collect::<Vec<_>>(),
            CROP_ASPECTS.to_vec(),
            "the ratio list is generated, not hard-coded"
        );
        // Apply fills exactly the names the canvas declares, with the payload's own numbers.
        let params = frame.params(&CropPayload {
            angle: -3.5,
            x: 0.25,
            y: 0.125,
            width: 0.5,
            height: 0.25,
        });
        assert_eq!(params["angle"], json!(-3.5));
        assert_eq!(params["x"], json!(0.25));
        assert_eq!(params["height"], json!(0.25));
        assert_eq!(params.len(), 5);
        // A crop frame is not a point pick, and an unavailable module declares no frame.
        assert!(point_pick(&modules).is_none());
        let unavailable = vec![ModuleDescriptor {
            availability: Availability::Unavailable {
                reason: "test".into(),
            },
            ..crop_descriptor()
        }];
        assert!(crop_frame(&unavailable).is_none());
    }

    #[test]
    fn a_draft_opens_on_the_existing_crop_layer_and_its_truncated_input_stage() {
        let payload = CropPayload {
            angle: 7.0,
            x: 0.2,
            y: 0.25,
            width: 0.4,
            height: 0.3,
        };
        let earlier = lightwell_core::Layer::pixel(0, 0, [1, 2, 3]);
        let crop = crop_layer(payload);
        let (mut editor, catalog, _, _) = opened(
            vec![earlier, crop.clone(), crop_layer(CropPayload::NEUTRAL)],
            4,
        );
        // Only the first crop layer is the one being edited.
        let _ = editor.update(Message::Crop(CropMessage::Start));
        let pending = editor.crop_pending.clone().expect("a pending draft");
        assert_eq!(pending.layer, Some(crop.id.clone()));
        assert_eq!(pending.layer_index, 1, "the preview truncates to one layer");
        assert_eq!(pending.payload, Some(payload));
        assert_eq!(pending.base_revision, 4);
        assert!(!pending.reapply);
        assert!(editor.crop.is_none(), "the draft waits for its input stage");

        editor.open_draft(CropStage {
            width: 480,
            height: 320,
            angle: 0.0,
        });
        let draft = editor.crop.as_ref().expect("an opened draft");
        assert_eq!(draft.layer, Some(crop.id));
        assert_eq!(draft.layer_index, 1);
        assert_eq!(draft.base_revision, 4);
        assert_eq!(draft.stage.angle, 7.0);
        assert_eq!(editor.crop_angle, "7");
        // Reopening shows exactly the rectangle the payload committed.
        let stage = CropStage {
            width: 480,
            height: 320,
            angle: 7.0,
        };
        assert_eq!(
            draft.output().expect("a valid draft"),
            payload.output_rect(&stage).expect("a valid payload")
        );
        assert_eq!(editor.snapshot()["crop"]["layer_index"], json!(1));
        finish(editor, catalog);
    }

    #[test]
    fn a_stack_without_a_crop_layer_drafts_a_neutral_crop_at_the_end() {
        let (mut editor, catalog, _, _) =
            opened(vec![lightwell_core::Layer::pixel(0, 0, [9, 9, 9])], 2);
        let _ = editor.update(Message::Crop(CropMessage::Start));
        let pending = editor.crop_pending.clone().expect("a pending draft");
        assert_eq!(pending.layer, None);
        assert_eq!(pending.layer_index, 1, "the whole stack is the input stage");
        editor.open_draft(CropStage {
            width: 480,
            height: 320,
            angle: 0.0,
        });
        let draft = editor.crop.as_ref().expect("an opened draft");
        assert!(draft.layer.is_none());
        assert_eq!(draft.payload(), CropPayload::NEUTRAL);
        finish(editor, catalog);
    }

    #[test]
    fn an_unreadable_crop_payload_refuses_the_draft_and_keeps_the_layer() {
        let mut broken = crop_layer(CropPayload::NEUTRAL);
        broken.payload = json!({"angle":"sideways"});
        let (mut editor, catalog, _, _) = opened(vec![broken], 1);
        let _ = editor.update(Message::Crop(CropMessage::Start));
        assert_eq!(
            editor.crop_pending.as_ref().map(|pending| pending.payload),
            Some(None)
        );
        editor.open_draft(CropStage {
            width: 480,
            height: 320,
            angle: 0.0,
        });
        assert!(editor.crop.is_none(), "no neutral crop replaced the layer");
        assert!(
            editor.status.contains("cannot be read"),
            "{}",
            editor.status
        );
        finish(editor, catalog);
    }

    #[test]
    fn every_draft_change_is_reachable_as_a_message() {
        let (mut editor, catalog, _, _) = opened(Vec::new(), 3);
        let _ = editor.update(Message::Crop(CropMessage::Start));
        editor.open_draft(CropStage {
            width: 480,
            height: 320,
            angle: 0.0,
        });
        let start = editor.crop.as_ref().expect("a draft").rect;

        // A pointer gesture: begin, drag, end. Nothing changes until the drag arrives.
        let _ = editor.update(Message::Crop(CropMessage::Pointer(CropPointer::Begin {
            handle: Handle::Corner(crate::crop_draft::Corner::TopLeft),
            x: 0.0,
            y: 0.0,
        })));
        assert_eq!(editor.crop.as_ref().expect("a draft").rect, start);
        let _ = editor.update(Message::Crop(CropMessage::Pointer(CropPointer::Drag {
            x: 80.0,
            y: 60.0,
            option: false,
        })));
        let _ = editor.update(Message::Crop(CropMessage::Pointer(CropPointer::End)));
        let dragged = editor.crop.as_ref().expect("a draft").rect;
        assert_eq!((dragged.x, dragged.y), (80.0, 60.0));

        // The angle field and its nudges.
        let _ = editor.update(Message::Crop(CropMessage::AngleText("11.5".into())));
        let _ = editor.update(Message::Crop(CropMessage::SubmitAngle));
        assert_eq!(editor.crop.as_ref().expect("a draft").stage.angle, 11.5);
        let _ = editor.update(Message::Crop(CropMessage::NudgeAngle(-ANGLE_STEP)));
        assert_eq!(editor.crop.as_ref().expect("a draft").stage.angle, 11.0);
        assert_eq!(editor.crop_angle, "11");
        let _ = editor.update(Message::Crop(CropMessage::AngleText("sideways".into())));
        let _ = editor.update(Message::Crop(CropMessage::SubmitAngle));
        assert_eq!(editor.crop.as_ref().expect("a draft").stage.angle, 11.0);
        assert!(editor.status.contains("Angle must be"), "{}", editor.status);
        let _ = editor.update(Message::Crop(CropMessage::AngleText("0".into())));
        let _ = editor.update(Message::Crop(CropMessage::SubmitAngle));

        // Ratio presets, swap, lock and the custom extents, all by index into the declared list.
        let index = |option: &str| {
            CROP_ASPECTS
                .iter()
                .position(|candidate| *candidate == option)
                .expect("a declared option")
        };
        let _ = editor.update(Message::Crop(CropMessage::Preset(index("16:9"))));
        let draft = editor.crop.as_ref().expect("a draft");
        assert_eq!(draft.preset, "16:9");
        assert_eq!(draft.aspect.ratio(), Some(16.0 / 9.0));
        let _ = editor.update(Message::Crop(CropMessage::Swap));
        assert_eq!(
            editor.crop.as_ref().expect("a draft").aspect.ratio(),
            Some(9.0 / 16.0)
        );
        let _ = editor.update(Message::Crop(CropMessage::Lock));
        assert_eq!(editor.crop.as_ref().expect("a draft").aspect.ratio(), None);
        let _ = editor.update(Message::Crop(CropMessage::CustomWidth("5".into())));
        let _ = editor.update(Message::Crop(CropMessage::CustomHeight("4".into())));
        let _ = editor.update(Message::Crop(CropMessage::Preset(index("custom"))));
        assert_eq!(
            editor.crop.as_ref().expect("a draft").aspect.ratio(),
            Some(1.25)
        );
        let _ = editor.update(Message::Crop(CropMessage::CustomHeight("none".into())));
        let _ = editor.update(Message::Crop(CropMessage::Preset(index("1:1"))));
        let _ = editor.update(Message::Crop(CropMessage::Preset(index("custom"))));
        assert_eq!(
            editor.crop.as_ref().expect("a draft").aspect.ratio(),
            Some(1.0),
            "an unreadable custom extent changes nothing"
        );

        // The modifier and guide state the canvas reads is app state, reachable by message.
        for (message, read) in [
            (CropMessage::Option(true), true),
            (CropMessage::Option(false), false),
        ] {
            let _ = editor.update(Message::Crop(message));
            assert_eq!(editor.crop_option, read);
        }
        let _ = editor.update(Message::Crop(CropMessage::Space(true)));
        assert!(editor.crop_space);
        let _ = editor.update(Message::Crop(CropMessage::Guide(true)));
        assert!(editor.crop_guide);

        let _ = editor.update(Message::Crop(CropMessage::Cancel));
        assert!(editor.crop.is_none());
        assert!(editor.draft_photo.is_none());
        assert!(!editor.crop_guide, "cancelling leaves no guide mode on");
        assert_eq!(
            editor.snapshot()["crop"],
            json!({"drafting":false,"pending":false})
        );
        finish(editor, catalog);
    }

    #[test]
    fn apply_builds_the_declared_request_against_the_drafts_own_revision() {
        let (mut editor, catalog, asset, _) = opened(Vec::new(), 6);
        let _ = editor.update(Message::Crop(CropMessage::Start));
        editor.open_draft(CropStage {
            width: 480,
            height: 320,
            angle: 0.0,
        });
        let _ = editor.update(Message::Crop(CropMessage::Pointer(CropPointer::Begin {
            handle: Handle::Corner(crate::crop_draft::Corner::TopLeft),
            x: 0.0,
            y: 0.0,
        })));
        let _ = editor.update(Message::Crop(CropMessage::Pointer(CropPointer::Drag {
            x: 48.0,
            y: 32.0,
            option: false,
        })));
        let _ = editor.update(Message::Crop(CropMessage::Pointer(CropPointer::End)));
        let payload = editor.crop.as_ref().expect("a draft").payload();
        let (method, request, request_id) = editor
            .crop_request()
            .expect("a request")
            .expect("a valid draft");
        assert_eq!(method, "edit.crop");
        assert_eq!(request["asset_id"], json!(asset));
        assert_eq!(request["mutation"]["expected_revision"], json!(6));
        assert_eq!(request["mutation"]["actor"], json!(ACTOR));
        assert_eq!(request["mutation"]["request_id"], json!(request_id));
        assert_eq!(request["angle"], json!(payload.angle));
        assert_eq!(request["x"], json!(payload.x));
        assert_eq!(request["width"], json!(payload.width));
        assert!(request.get("aspect").is_none(), "only the declared five");
        finish(editor, catalog);
    }

    #[test]
    fn an_external_commit_marks_the_draft_conflicted_and_reapply_rebases_it() {
        let (mut editor, catalog, asset, _) = opened(Vec::new(), 1);
        let _ = editor.update(Message::Crop(CropMessage::Start));
        editor.open_draft(CropStage {
            width: 480,
            height: 320,
            angle: 0.0,
        });
        let _ = editor.update(Message::Crop(CropMessage::AngleText("6".into())));
        let _ = editor.update(Message::Crop(CropMessage::SubmitAngle));
        let composed = editor.crop.as_ref().expect("a draft").rect;

        // Somebody else committed: the draft survives and says so, and Apply is refused.
        let newer = entry(&asset, 9, None);
        let refresh = refresh_for(&asset, &newer, vec![newer.clone()], &[&newer], false);
        let _ = editor.update(Message::Synced(Ok(SyncResult::Changed(Box::new(refresh)))));
        let draft = editor.crop.as_ref().expect("the draft is kept");
        assert!(draft.conflicted);
        assert_eq!(draft.rect, composed, "the composition is untouched");
        assert!(editor.crop_request().is_none(), "Apply is refused");
        assert_eq!(editor.snapshot()["crop"]["conflicted"], json!(true));

        // Reapply re-reads the stack and rebases onto the new revision and input stage.
        editor.busy = false;
        let _ = editor.update(Message::Crop(CropMessage::Reapply));
        let pending = editor.crop_pending.clone().expect("a pending rebase");
        assert!(pending.reapply);
        assert_eq!(pending.base_revision, 9);
        editor.open_draft(CropStage {
            width: 480,
            height: 320,
            angle: 0.0,
        });
        let draft = editor.crop.as_ref().expect("the rebased draft");
        assert!(!draft.conflicted);
        assert_eq!(draft.base_revision, 9);
        assert_eq!(draft.stage.angle, 6.0, "the angle survives a rebase");
        assert!(editor.crop_request().is_some(), "Apply is possible again");
        finish(editor, catalog);
    }

    #[test]
    fn the_drafts_own_apply_ends_it_and_a_failed_apply_keeps_it() {
        let (mut editor, catalog, asset, _) = opened(Vec::new(), 2);
        let _ = editor.update(Message::Crop(CropMessage::Start));
        editor.open_draft(CropStage {
            width: 480,
            height: 320,
            angle: 0.0,
        });
        // A stale revision comes back as a conflict: the draft is kept and marked.
        editor.crop_applying = Some("desktop-1".into());
        let _ = editor.update(Message::Refreshed(Err("conflict: stale revision".into())));
        assert!(editor.crop.as_ref().expect("the draft is kept").conflicted);
        assert!(editor.crop_applying.is_none());

        // The draft's own successful Apply ends it and drops the extra texture.
        editor.crop.as_mut().expect("a draft").conflicted = false;
        editor.crop_applying = Some("desktop-2".into());
        let newer = entry(&asset, 5, None);
        let refresh = refresh_for(&asset, &newer, vec![newer.clone()], &[&newer], false);
        let _ = editor.update(Message::Refreshed(Ok(Box::new(refresh))));
        assert!(editor.crop.is_none());
        assert!(editor.draft_photo.is_none());
        assert!(editor.status.contains("Crop applied"), "{}", editor.status);
        finish(editor, catalog);
    }

    #[test]
    fn a_history_preview_pauses_the_draft_without_discarding_it() {
        let (mut editor, catalog, asset, entry_id) = opened(Vec::new(), 1);
        let _ = editor.update(Message::Crop(CropMessage::Start));
        editor.open_draft(CropStage {
            width: 480,
            height: 320,
            angle: 0.0,
        });
        let composed = editor.crop.as_ref().expect("a draft").rect;
        let mut session = ClientSession {
            revision: 3,
            ..ClientSession::default()
        };
        session.preview.selection = HistorySelection::Entry(entry_id);
        let _ = editor.update(Message::SessionUpdated(Ok((session, 1))));
        assert!(!editor.session.preview.can_edit());
        assert!(
            editor.crop.is_some(),
            "selecting a historical state keeps the draft"
        );
        assert!(!editor.drafting(), "the plain historical preview is shown");
        assert_eq!(editor.snapshot()["crop"]["paused"], json!(true));
        // Nothing can be applied or started while previewing history.
        let _ = editor.update(Message::Crop(CropMessage::Apply));
        assert!(editor.crop.is_some());
        assert!(editor.crop_applying.is_none());
        assert_eq!(editor.crop.as_ref().expect("a draft").rect, composed);
        let _ = std::hint::black_box(&asset);
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
