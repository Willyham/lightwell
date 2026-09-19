use iced::{
    Element, Length, Subscription, Task,
    widget::{button, column, container, image, row, text},
};
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
                    "Lightwell: --open JPEG (repeatable with evidence) --evidence-dir NEW_DIRECTORY --window-size WIDTH HEIGHT"
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
    Ok(config)
}
struct Viewer {
    loader: Loader,
    photo: Option<image::Handle>,
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
    started: Instant,
    events: Vec<Value>,
    frames: Vec<Value>,
    backend: Option<Value>,
    errors: bool,
    error_code: Option<String>,
}
#[derive(Debug, Clone)]
enum Message {
    Open,
    Picked(Option<PathBuf>),
    Poll,
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
            started: Instant::now(),
            events: Vec::new(),
            frames: Vec::new(),
            backend: None,
            errors: false,
            error_code: None,
        };
        app.event("startup",json!({"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"version":env!("CARGO_PKG_VERSION")}));
        if let Some(path) = app.config.files.pop_front() {
            app.request(path);
        } else {
            app.capture_pending = app.config.evidence.is_some();
        }
        (app, iced::system::information().map(Message::Info))
    }
    fn event(&mut self, name: &str, detail: Value) {
        let value = json!({"event":name,"elapsed_ms":self.started.elapsed().as_secs_f64()*1000.,"generation":self.generation,"detail":detail});
        if self.config.evidence.is_some() && self.events.len() < 256 {
            self.events.push(value);
        }
    }
    fn request(&mut self, path: PathBuf) {
        self.generation += 1;
        self.loader.request(path);
        self.error_code = None;
        self.phase = "loading";
        self.status = "Loading photograph…".into();
        self.capture_pending = false;
        self.event("open_requested", json!({}));
    }
    fn snapshot(&self) -> Value {
        json!({"phase":self.phase,"requested_generation":self.generation,"displayed_generation":self.displayed,"source_dimensions":self.dimensions,"preview_dimensions":self.preview,"backend":self.backend,"status":self.status,"error_code":self.error_code})
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
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
            Message::Poll => {
                if self.config.evidence.is_some()
                    && self.started.elapsed() > Duration::from_secs(25)
                {
                    eprintln!("Evidence deadline exceeded; inspect retained subprocess output");
                    std::process::exit(3);
                }
                if let Some(result) = self.loader.poll() {
                    match result {
                        Ok(photo) => {
                            self.status = format!("{} × {} · Fit", photo.width, photo.height);
                            self.phase = "ready";
                            self.displayed = self.generation;
                            self.dimensions = Some((photo.width, photo.height));
                            self.preview = Some((photo.preview_width, photo.preview_height));
                            self.event("decoded",json!({"decode_ms":photo.decode_ms,"source_dimensions":self.dimensions,"preview_dimensions":self.preview}));
                            self.photo = Some(image::Handle::from_rgba(
                                photo.preview_width,
                                photo.preview_height,
                                photo.rgba,
                            ));
                        }
                        Err(error) => {
                            self.phase = "error";
                            self.error_code = Some(
                                error
                                    .split(':')
                                    .next()
                                    .unwrap_or("decode-error")
                                    .to_string(),
                            );
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
                    json!({"displayed_generation":self.displayed}),
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
                    let events = std::mem::take(&mut self.events);
                    let state = self.snapshot();
                    let frames = std::mem::take(&mut self.frames);
                    let result = json!({"status":"captured","had_input_errors":self.errors,"frames":frames,"elapsed_ms":self.started.elapsed().as_secs_f64()*1000.,"build_version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"arch":std::env::consts::ARCH});
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
                                std::fs::write(
                                    dir.join("events.jsonl"),
                                    events.iter().map(|v| format!("{v}\n")).collect::<String>(),
                                )?;
                                Ok(())
                            };
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
            Some(handle) => image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(iced::ContentFit::Contain)
                .into(),
            None => container(text("Open a photograph").size(24))
                .center(Length::Fill)
                .into(),
        };
        let open = button("Open image").on_press_maybe(
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
    if let Err(error) = iced::application(
        move || Viewer::new(config.clone()),
        Viewer::update,
        Viewer::view,
    )
    .title("Lightwell")
    .window_size(size)
    .theme(iced::Theme::Dark)
    .subscription(Viewer::subscription)
    .run()
    {
        eprintln!("Could not start Lightwell: {error}. Check desktop session and graphics driver.");
        std::process::exit(1);
    }
}
