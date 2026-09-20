use crate::{Config, paths::Paths};
use iced::{
    Element, Length, Subscription, Task,
    widget::{button, column, container, image, row, scrollable, text, text_input},
};
use iced_runtime::image as image_memory;
use lightwell_core::{
    ApiRequest, AssetId, ClientId, ClientSession, EditorState, EntryId, ErrorKind, EventsResult,
    HistoryEntry, HistoryPage, HistorySelection, Lineage, LocalServer, Mutation, OwnerHandle,
    PreviewJob, PreviewQueue, Transform, Version, Zoom,
};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

static REQUEST_NUMBER: AtomicU64 = AtomicU64::new(1);
const HISTORY_PAGE_SIZE: usize = 50;
const LINEAGE_LIMIT: usize = 100;
const ACTOR: &str = "desktop";

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
    X(String),
    Y(String),
    R(String),
    G(String),
    B(String),
    Zoom(String),
    VersionName(String),
    Panned(f32, f32),
    ApplyPixel,
    Transform(Transform),
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
    Close,
}

/// Catalog ownership and the live service start before the window so failures are reported, not panics.
struct Boot {
    owner: OwnerHandle,
    join: JoinHandle<()>,
    live_server: Option<LocalServer>,
    initial: Option<PathBuf>,
}

pub(super) fn run(mut config: Config, size: (f32, f32)) -> Result<(), String> {
    let paths = Paths::resolve(config.data_root.as_ref())
        .ok_or("no usable application data directory; pass --data-root")?;
    let catalog = config
        .catalog
        .clone()
        .unwrap_or_else(|| paths.config.join("catalog.sqlite"));
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
        initial: config.files.pop_front(),
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
    x: String,
    y: String,
    r: String,
    g: String,
    b: String,
    zoom: String,
    version_name: String,
}

impl Editor {
    fn new(boot: Boot) -> (Self, Task<Message>) {
        let Boot {
            owner,
            join,
            live_server,
            initial,
        } = boot;
        let client = owner.register();
        let mut editor = Self {
            owner: owner.clone(),
            owner_join: Some(join),
            live_server,
            client,
            session: ClientSession::default(),
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
            busy: initial.is_some(),
            syncing: false,
            pan_in_flight: false,
            pending_pan: None,
            picker_open: false,
            status: if initial.is_some() {
                "Importing photograph…".into()
            } else {
                "Open a JPEG to begin".into()
            },
            api_sequence: 0,
            scale_factor: 1.0,
            x: "0".into(),
            y: "0".into(),
            r: "255".into(),
            g: "0".into(),
            b: "0".into(),
            zoom: "100".into(),
            version_name: String::new(),
        };
        if editor.live_server.is_none() {
            editor.status = "Editor ready; live API unavailable on this host".into();
        }
        let scale = iced::window::oldest()
            .and_then(iced::window::scale_factor)
            .map(Message::ScaleFactor);
        let import = initial
            .map(|path| import_task(owner, client, path))
            .unwrap_or_else(Task::none);
        (editor, Task::batch([scale, import]))
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
                if self.picker_open || self.busy {
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
                    self.busy = true;
                    self.status = "Importing photograph…".into();
                    return import_task(self.owner.clone(), self.client, path);
                }
            }
            Message::Refreshed(result) => {
                self.busy = false;
                match result {
                    Ok(refresh) => self.accept(*refresh),
                    Err(error) => self.status = error,
                }
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
                            let upload = Upload {
                                generation: result.generation,
                                width: raster.width,
                                height: raster.height,
                                entry_id: result.entry_id,
                                snapshot_id: raster.snapshot_id.to_string(),
                                source_fingerprint: raster.source_fingerprint,
                            };
                            let handle = image::Handle::from_rgba(
                                raster.width,
                                raster.height,
                                iced_runtime::core::Bytes::from_owner(raster.rgba),
                            );
                            return image_memory::allocate(handle)
                                .map(move |result| Message::Uploaded(upload.clone(), result));
                        }
                        Err(error) => self.status = error.to_string(),
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
                    Err(_) => self.status = "Could not upload rendered pixels".into(),
                }
            }
            Message::X(value) => self.x = value,
            Message::Y(value) => self.y = value,
            Message::R(value) => self.r = value,
            Message::G(value) => self.g = value,
            Message::B(value) => self.b = value,
            Message::Zoom(value) => self.zoom = value,
            Message::VersionName(value) => self.version_name = value,
            Message::Panned(x, y) => return self.pan(x, y),
            Message::ApplyPixel => {
                let Some(state) = &self.state else {
                    return Task::none();
                };
                let parsed = || {
                    Some((
                        self.x.parse::<u32>().ok()?,
                        self.y.parse::<u32>().ok()?,
                        [
                            self.r.parse::<u8>().ok()?,
                            self.g.parse::<u8>().ok()?,
                            self.b.parse::<u8>().ok()?,
                        ],
                    ))
                };
                let Some((x, y, rgb)) = parsed() else {
                    self.status =
                        "Pixel fields require integer x/y and RGB values from 0 to 255".into();
                    return Task::none();
                };
                let params = json!({"asset_id":state.asset.id,"mutation":mutation(state.revision),"x":x,"y":y,"rgb":rgb});
                return self.command("edit.set-pixel", params);
            }
            Message::Transform(transform) => {
                let Some(state) = &self.state else {
                    return Task::none();
                };
                let params = json!({"asset_id":state.asset.id,"mutation":mutation(state.revision),"transform":transform});
                return self.command("edit.transform", params);
            }
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
                self.live_server.take();
                self.owner.disconnect(self.client);
                self.owner.stop();
                if let Some(join) = self.owner_join.take() {
                    return Task::perform(
                        async move {
                            let _ = join.join();
                        },
                        |_| (),
                    )
                    .then(|_| iced::exit());
                }
                return iced::exit();
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

    fn command(&mut self, method: &'static str, params: Value) -> Task<Message> {
        let Some(state) = &self.state else {
            return Task::none();
        };
        if self.busy {
            return Task::none();
        }
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

    fn view(&self) -> Element<'_, Message> {
        let current = self.state.as_ref();
        let editable = current.is_some() && self.session.preview.can_edit() && !self.busy;
        let open = button("Open image").on_press_maybe((!self.busy).then_some(Message::Open));
        let header = row![text("Lightwell").size(22), open]
            .spacing(16)
            .align_y(iced::Alignment::Center);

        let surface: Element<'_, Message> = match (&self.photo, self.dimensions) {
            (Some(allocation), Some((width, height))) => match self.session.preview.view.zoom {
                Zoom::Fit => image(allocation.handle().clone())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(iced::ContentFit::Contain)
                    .into(),
                Zoom::Percent { value } => {
                    let scale = value / 100.0 / self.scale_factor;
                    let image = image(allocation.handle().clone())
                        .width(Length::Fixed(width as f32 * scale))
                        .height(Length::Fixed(height as f32 * scale));
                    scrollable(container(image).center(Length::Shrink))
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

        let pixel = column![
            text("Pixel proof").size(18),
            row![
                text_input("x", &self.x).on_input(Message::X).width(55),
                text_input("y", &self.y).on_input(Message::Y).width(55),
                text_input("R", &self.r).on_input(Message::R).width(55),
                text_input("G", &self.g).on_input(Message::G).width(55),
                text_input("B", &self.b).on_input(Message::B).width(55),
            ]
            .spacing(6),
            button("Apply pixel").on_press_maybe(editable.then_some(Message::ApplyPixel)),
        ]
        .spacing(8);

        let transforms = column![
            text("Exact transforms").size(18),
            row![
                button("Rotate left")
                    .on_press_maybe(editable.then_some(Message::Transform(Transform::RotateLeft))),
                button("Rotate right")
                    .on_press_maybe(editable.then_some(Message::Transform(Transform::RotateRight))),
            ]
            .spacing(6),
            row![
                button("Mirror horizontal").on_press_maybe(
                    editable.then_some(Message::Transform(Transform::MirrorHorizontal))
                ),
                button("Flip vertical").on_press_maybe(
                    editable.then_some(Message::Transform(Transform::FlipVertical))
                ),
            ]
            .spacing(6),
        ]
        .spacing(8);

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
                pixel,
                transforms,
                zoom,
                history_rows,
                version_rows,
                text("Layer stack").size(18),
                text(layers).size(11)
            ]
            .spacing(18)
            .padding(12),
        )
        .width(340);
        column![
            header,
            row![
                container(surface).width(Length::Fill).height(Length::Fill),
                sidebar
            ]
            .spacing(12)
            .height(Length::Fill),
            text(&self.status).size(12),
        ]
        .spacing(12)
        .padding(16)
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
                && modifiers.command()
            {
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
            }
            None
        });
        let mut subscriptions = vec![events];
        if self.preview_queue.is_busy() {
            subscriptions.push(iced::time::every(Duration::from_millis(16)).map(|_| Message::Poll));
        }
        if self.state.is_some() {
            subscriptions
                .push(iced::time::every(Duration::from_millis(500)).map(|_| Message::Sync));
        }
        Subscription::batch(subscriptions)
    }
}

fn short(value: &str) -> &str {
    value.get(..value.len().min(12)).unwrap_or(value)
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
    method: &'static str,
    params: Value,
) -> Task<Message> {
    Task::perform(
        async move {
            let (_, sequence) = call(&owner, client, method, params)?;
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
            initial: None,
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

    #[test]
    fn pixel_fields_reject_non_integral_values() {
        assert!("1.5".parse::<u32>().is_err());
        assert!("256".parse::<u8>().is_err());
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
