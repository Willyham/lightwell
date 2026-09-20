mod diagnostics;
mod editor_app;
mod paths;
use diagnostics::Diagnostics;
use iced::{
    Element, Length, Subscription, Task,
    widget::{button, column, container, image, row, text},
};
use iced_runtime::image as image_memory;
use lightwell_core::Loader;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone, Default)]
struct Config {
    files: VecDeque<PathBuf>,
    evidence: Option<PathBuf>,
    size: Option<(f32, f32)>,
    data_root: Option<PathBuf>,
    catalog: Option<PathBuf>,
    diagnostics: Option<Diagnostics>,
    run_id: String,
}
fn arguments() -> Result<Config, String> {
    let mut config = Config::default();
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--open") => config
                .files
                .push_back(args.next().ok_or("--open requires a path")?.into()),
            Some("--evidence-dir") => {
                config.evidence = Some(args.next().ok_or("--evidence-dir requires a path")?.into())
            }
            Some("--data-root") => {
                config.data_root = Some(args.next().ok_or("--data-root requires a path")?.into());
            }
            Some("--catalog") => {
                config.catalog = Some(args.next().ok_or("--catalog requires a path")?.into());
            }
            Some("--window-size") => {
                let mut number = || {
                    args.next()
                        .and_then(|v| v.to_str().and_then(|s| s.parse::<u32>().ok()))
                        .filter(|n| (320..=4096).contains(n))
                        .ok_or("Window dimensions must be 320..4096")
                };
                config.size = Some((number()? as f32, number()? as f32));
            }
            Some("--help") => {
                println!(
                    "Lightwell: --open JPEG --catalog CATALOG --data-root DIRECTORY [--evidence-dir NEW_DIRECTORY --window-size WIDTH HEIGHT]"
                );
                std::process::exit(0)
            }
            _ => return Err("Unknown argument; use --help".into()),
        }
    }
    if config.files.len() > 16 {
        return Err("At most 16 evidence requests are supported per run".into());
    }
    if config.files.len() > 1 && config.evidence.is_none() {
        return Err("Repeated --open requires --evidence-dir".into());
    }
    if let Some(path) = &config.evidence {
        if path.exists() {
            return Err("Evidence directory must be new to prevent stale evidence".into());
        }
        std::fs::create_dir_all(path)
            .map_err(|e| format!("Cannot create evidence directory: {}", e.kind()))?;
    }
    config.run_id = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let resolved = paths::Paths::resolve(config.data_root.as_ref());
    // Config and cache are deliberately not created until they have real work.
    if let Some(paths) = &resolved {
        debug_assert!(paths.config != paths.cache);
    }
    let log_dir = config.evidence.clone().or_else(|| {
        config
            .data_root
            .as_ref()
            .and(resolved.as_ref().map(|p| p.logs.clone()))
    });
    if let Some(dir) = log_dir {
        let start = || -> std::io::Result<Diagnostics> {
            std::fs::create_dir_all(&dir)?;
            Diagnostics::start(&dir.join("events.jsonl"))
        };
        match start() {
            Ok(log) => {
                log.panic_hook(config.run_id.clone());
                config.diagnostics = Some(log);
            }
            Err(error) if config.evidence.is_some() => {
                return Err(format!(
                    "diagnostics: cannot initialize log: {}",
                    error.kind()
                ));
            }
            Err(error) => eprintln!(
                "diagnostics: logging unavailable: {}; viewing continues",
                error.kind()
            ),
        }
    }
    Ok(config)
}
struct Viewer {
    loader: Loader,
    photo: Option<image_memory::Allocation>,
    uploading: bool,
    status: String,
    phase: &'static str,
    config: Config,
    generation: u64,
    displayed: u64,
    dimensions: Option<(u32, u32)>,
    preview: Option<(u32, u32)>,
    capture_pending: bool,
    saving: bool,
    picker_open: bool,
    open_focused: bool,
    started: Instant,
    orientation: Option<u8>,
    request_started: Instant,
    frames: Vec<Value>,
    backend: Option<Value>,
    errors: bool,
    error_code: Option<String>,
}
#[derive(Debug, Clone)]
struct Upload {
    generation: u64,
    dimensions: (u32, u32),
    preview: (u32, u32),
    orientation: u8,
    started: Instant,
}
#[derive(Debug, Clone)]
enum Message {
    Open,
    Close,
    FocusOpen,
    ActivateOpen,
    Picked(Option<PathBuf>),
    Poll,
    Uploaded(
        Upload,
        Result<image_memory::Allocation, image_memory::Error>,
    ),
    Capture,
    Captured(iced::window::Screenshot),
    Saved(Result<Value, String>),
    Info(iced::system::Information),
}
impl Viewer {
    fn new(config: Config) -> (Self, Task<Message>) {
        let mut app = Self {
            loader: Loader::default(),
            photo: None,
            uploading: false,
            status: "Open a JPEG to begin".into(),
            phase: "empty",
            config,
            generation: 0,
            displayed: 0,
            dimensions: None,
            preview: None,
            capture_pending: false,
            saving: false,
            picker_open: false,
            open_focused: false,
            started: Instant::now(),
            orientation: None,
            request_started: Instant::now(),
            frames: Vec::new(),
            backend: None,
            errors: false,
            error_code: None,
        };
        app.event("startup",json!({"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"version":env!("CARGO_PKG_VERSION"),"debug_assertions":cfg!(debug_assertions)}));
        if let Some(path) = app.config.files.pop_front() {
            app.request(path);
        } else {
            app.capture_pending = app.config.evidence.is_some();
        }
        (app, iced::system::information().map(Message::Info))
    }
    fn event(&mut self, name: &str, detail: Value) {
        let value = json!({"event":name,"run_id":self.config.run_id,"build_version":env!("CARGO_PKG_VERSION"),"elapsed_ms":self.started.elapsed().as_secs_f64()*1000.,"request_id":self.generation,"generation":self.generation,"detail":detail});
        if let Some(log) = &self.config.diagnostics {
            log.event(value);
        } else {
            eprintln!("{value}");
        }
    }

    fn request(&mut self, path: PathBuf) {
        self.generation += 1;
        self.request_started = Instant::now();
        self.loader.request(path);
        self.error_code = None;
        self.phase = "loading";
        self.status = "Loading photograph…".into();
        self.capture_pending = false;
        self.event("open_requested", json!({}));
    }
    fn snapshot(&self) -> Value {
        json!({"run_id":self.config.run_id,"orientation":self.orientation,"phase":self.phase,"requested_generation":self.generation,"displayed_generation":self.displayed,"source_dimensions":self.dimensions,"preview_dimensions":self.preview,"backend":self.backend,"status":self.status,"error_code":self.error_code})
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FocusOpen => self.open_focused = true,
            Message::ActivateOpen => {
                if self.open_focused {
                    return self.update(Message::Open);
                }
            }
            Message::Close => {
                self.event("shutdown", json!({"while_loading":self.loader.loading()}));
                let log = self.config.diagnostics.clone();
                return Task::perform(
                    async move {
                        if let Some(log) = log {
                            log.finish();
                        }
                    },
                    |_| (),
                )
                .then(|_| iced::exit());
            }
            Message::Info(info) => {
                self.backend =
                    Some(json!({"backend":info.graphics_backend,"adapter":info.graphics_adapter}));
                self.event("backend", self.backend.clone().unwrap());
            }
            Message::Open => {
                if self.picker_open || self.config.evidence.is_some() {
                    return Task::none();
                }
                self.picker_open = true;
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("JPEG", &["jpg", "jpeg"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::Picked,
                );
            }
            Message::Picked(path) => {
                self.picker_open = false;
                if let Some(path) = path {
                    self.request(path);
                }
            }
            Message::Uploaded(upload, result) => {
                self.uploading = false;
                if upload.generation != self.generation {
                    return Task::none();
                }
                match result {
                    Ok(allocation) => {
                        self.photo = Some(allocation);
                        self.dimensions = Some(upload.dimensions);
                        self.preview = Some(upload.preview);
                        self.orientation = Some(upload.orientation);
                        self.displayed = upload.generation;
                        self.phase = "ready";
                        self.status =
                            format!("{} × {} · Fit", upload.dimensions.0, upload.dimensions.1);
                        self.event("render_ready", json!({"upload_ms":upload.started.elapsed().as_secs_f64()*1000.,"displayed_generation":self.displayed}));
                    }
                    Err(_) => {
                        self.phase = "error";
                        self.errors = true;
                        self.error_code = Some(lightwell_core::ErrorKind::Render.code().into());
                        self.status =
                            "Could not prepare this image for display. Try a smaller JPEG.".into();
                        self.event("render_failed", json!({"error_code":"render"}));
                    }
                }
                self.capture_pending = self.config.evidence.is_some();
            }
            Message::Poll => {
                if self.config.evidence.is_some()
                    && self.started.elapsed() > Duration::from_secs(25)
                {
                    eprintln!("Evidence deadline exceeded; inspect retained subprocess output");
                    std::process::exit(3);
                }
                if !self.uploading
                    && let Some(result) = self.loader.poll()
                {
                    match result {
                        Ok(photo) => {
                            self.event("decoded",json!({"decode_ms":photo.decode_ms,"stages_ms":{"read":photo.timings.read_ms,"validate":photo.timings.validate_ms,"pixels":photo.timings.pixels_ms,"orient":photo.timings.orient_ms,"resize":photo.timings.resize_ms,"rgba":photo.timings.rgba_ms},"source_dimensions":[photo.width,photo.height],"preview_dimensions":[photo.preview_width,photo.preview_height]}));
                            let upload = Upload {
                                generation: self.generation,
                                dimensions: (photo.width, photo.height),
                                preview: (photo.preview_width, photo.preview_height),
                                orientation: photo.orientation,
                                started: Instant::now(),
                            };
                            self.uploading = true;
                            self.status = "Preparing photograph…".into();
                            let handle = image::Handle::from_rgba(
                                photo.preview_width,
                                photo.preview_height,
                                photo.rgba,
                            );
                            // Allocation completion guarantees the image can draw in the next frame.
                            // Keep the previous allocation until this generation is ready.
                            return image_memory::allocate(handle)
                                .map(move |result| Message::Uploaded(upload.clone(), result));
                        }
                        Err(error) => {
                            self.phase = "error";
                            self.error_code = Some(error.kind.code().to_string());
                            self.status = match self.error_code.as_deref() {
                                Some("read-error") => "Could not read this file. Check that it is available and readable.",
                                Some("resource-limit") => "This image exceeds the current size limit.",
                                Some("unsupported-profile") => "This JPEG uses an unsupported color profile. Choose an sRGB JPEG.",
                                Some("unsupported-color") | Some("unsupported-input") => "This JPEG format is not supported yet.",
                                _ => "Could not open this JPEG. The file may be incomplete or damaged.",
                            }.into();
                            self.errors = true;
                            self.event(
                                "open_failed",
                                json!({"message":self.status,"error_code":self.error_code}),
                            );
                        }
                    }
                    self.capture_pending = self.config.evidence.is_some();
                }
            }
            Message::Capture => {
                if !self.capture_pending || self.saving || self.backend.is_none() {
                    return Task::none();
                }
                self.capture_pending = false;
                self.saving = true;
                return iced::window::oldest()
                    .and_then(iced::window::screenshot)
                    .map(Message::Captured);
            }
            Message::Captured(shot) => {
                self.event(
                    "frame_captured",
                    json!({"displayed_generation":self.displayed,"request_to_capture_ms":self.request_started.elapsed().as_secs_f64()*1000.}),
                );
                let state = self.snapshot();
                let generation = self.generation;
                let dir = self.config.evidence.clone().expect("capture is opt-in");
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
                        let frame = json!({"file":name,"state":state,"capture_provenance":"window-renderer-readback","color":"sRGB","physical_size":[shot.size.width,shot.size.height],"scale":shot.scale_factor});
                        std::fs::write(
                            dir.join(format!("state-{generation}.json")),
                            serde_json::to_vec_pretty(&frame).unwrap(),
                        )
                        .map_err(|e| e.to_string())?;
                        Ok(frame)
                    },
                    Message::Saved,
                );
            }
            Message::Saved(result) => {
                self.saving = false;
                match result {
                    Ok(frame) => self.frames.push(frame),
                    Err(error) => {
                        eprintln!("Evidence write failed: {error}");
                        std::process::exit(4);
                    }
                }
                if let Some(path) = self.config.files.pop_front() {
                    self.request(path);
                } else {
                    self.event("shutdown", json!({}));
                    let dir = self.config.evidence.clone().unwrap();
                    let log = self.config.diagnostics.clone();
                    let state = self.snapshot();
                    let frames = std::mem::take(&mut self.frames);
                    let result = json!({"run_id":self.config.run_id,"status":"captured","had_input_errors":self.errors,"frames":frames,"elapsed_ms":self.started.elapsed().as_secs_f64()*1000.,"build_version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"arch":std::env::consts::ARCH});
                    return Task::perform(
                        async move {
                            let write = || -> std::io::Result<()> {
                                std::fs::write(
                                    dir.join("result.json"),
                                    serde_json::to_vec_pretty(&result).unwrap(),
                                )?;
                                std::fs::write(
                                    dir.join("state.json"),
                                    serde_json::to_vec_pretty(&state).unwrap(),
                                )?;
                                Ok(())
                            };
                            if let Some(log) = log
                                && !log.finish()
                            {
                                eprintln!("diagnostics: incomplete evidence log");
                                std::process::exit(4);
                            }
                            if let Err(error) = write() {
                                eprintln!("Evidence finalize failed: {error}");
                                std::process::exit(4);
                            }
                        },
                        |_| (),
                    )
                    .then(|_| iced::exit());
                }
            }
        }
        Task::none()
    }
    fn view(&self) -> Element<'_, Message> {
        let surface: Element<'_, Message> = match &self.photo {
            Some(allocation) => image(allocation.handle().clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(iced::ContentFit::Contain)
                .into(),
            None => container(text("Open a photograph").size(24))
                .center(Length::Fill)
                .into(),
        };
        let open = button("Open image")
            .style(|theme, status| {
                let mut style = button::primary(theme, status);
                if self.open_focused && !self.picker_open {
                    style.border.width = 2.;
                    style.border.color = iced::Color::WHITE;
                }
                style
            })
            .on_press_maybe(
                (!self.picker_open && self.config.evidence.is_none()).then_some(Message::Open),
            );
        column![
            row![text("Lightwell").size(22), open].spacing(24),
            container(surface).width(Length::Fill).height(Length::Fill),
            text(&self.status).size(14)
        ]
        .spacing(16)
        .padding(24)
        .into()
    }
    fn subscription(&self) -> Subscription<Message> {
        let mut list = vec![iced::event::listen_with(|event, _, _| {
            if matches!(
                event,
                iced::Event::Window(iced::window::Event::CloseRequested)
            ) {
                return Some(Message::Close);
            }
            if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { ref key, .. }) = event
            {
                match key {
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) => {
                        return Some(Message::FocusOpen);
                    }
                    iced::keyboard::Key::Named(
                        iced::keyboard::key::Named::Enter | iced::keyboard::key::Named::Space,
                    ) => return Some(Message::ActivateOpen),
                    _ => {}
                }
            }
            if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key, modifiers, ..
            }) = event
                && modifiers.command()
                && matches!(key,iced::keyboard::Key::Character(ref c) if c.eq_ignore_ascii_case("o"))
            {
                return Some(Message::Open);
            }
            None
        })];
        if self.loader.loading() || self.config.evidence.is_some() {
            list.push(iced::time::every(Duration::from_millis(16)).map(|_| Message::Poll));
        }
        if self.capture_pending {
            list.push(iced::window::frames().map(|_| Message::Capture));
        }
        Subscription::batch(list)
    }
}
fn main() {
    let config = arguments().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2)
    });
    let size = config.size.unwrap_or((960., 640.));
    if config.evidence.is_none() {
        if let Err(error) = editor_app::run(config, size) {
            eprintln!("Could not start Lightwell editor: {error}");
            std::process::exit(1);
        }
        return;
    }
    if let Err(error) = iced::application(
        move || Viewer::new(config.clone()),
        Viewer::update,
        Viewer::view,
    )
    .title("Lightwell")
    .window_size(size)
    .exit_on_close_request(false)
    .theme(iced::Theme::Dark)
    .subscription(Viewer::subscription)
    .run()
    {
        eprintln!("Could not start Lightwell: {error}. Check desktop session and graphics driver.");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod state_tests {
    use super::*;
    #[test]
    fn failed_replacement_and_cancel_preserve_displayed_state() {
        let (mut viewer, _) = Viewer::new(Config::default());
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/s0");
        viewer.dimensions = Some((320, 480));
        viewer.orientation = Some(6);
        viewer.displayed = 1;
        viewer.generation = 1;
        for file in ["invalid.jpg"] {
            viewer.request(fixture.join(file));
            let deadline = Instant::now() + Duration::from_secs(5);
            while viewer.loader.loading() {
                let _ = viewer.update(Message::Poll);
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
        }
        assert_eq!(viewer.phase, "error");
        assert_eq!(viewer.displayed, 1);
        assert_eq!(viewer.generation, 2);
        assert_eq!(viewer.dimensions, Some((320, 480)));
        assert_eq!(viewer.orientation, Some(6));
        assert_eq!(viewer.error_code.as_deref(), Some("invalid-input"));
        let before = viewer.snapshot();
        let _ = viewer.update(Message::Picked(None));
        assert_eq!(viewer.snapshot(), before);
        assert!(!viewer.loader.loading());
    }
    #[test]
    fn decoding_alone_does_not_claim_render_readiness() {
        let (mut viewer, _) = Viewer::new(Config::default());
        viewer.request(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/s0/orientation-6.jpg"),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while viewer.loader.loading() {
            let _ = viewer.update(Message::Poll);
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(viewer.uploading);
        assert_eq!(viewer.phase, "loading");
        assert_eq!(viewer.displayed, 0);
        assert!(!viewer.capture_pending);
        viewer.generation += 1;
        let before = viewer.snapshot();
        let _ = viewer.update(Message::Uploaded(
            Upload {
                generation: 1,
                dimensions: (320, 480),
                preview: (320, 480),
                orientation: 6,
                started: Instant::now(),
            },
            Err(image_memory::Error::Unsupported),
        ));
        assert_eq!(
            viewer.snapshot(),
            before,
            "obsolete upload must not publish errors or pixels"
        );
        assert!(!viewer.uploading);
    }
    #[test]
    fn evidence_mode_disables_manual_open() {
        let (mut viewer, _) = Viewer::new(Config {
            evidence: Some(PathBuf::from("unused")),
            ..Config::default()
        });
        let _ = viewer.update(Message::Open);
        assert!(!viewer.picker_open);
        assert_eq!(viewer.generation, 0);
    }
}
