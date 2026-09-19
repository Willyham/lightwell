use iced::{
    Element, Length, Subscription, Task,
    widget::{button, column, container, image, row, text},
};
use lightwell_s0_probes::Loader;
use std::{path::PathBuf, time::Duration};

#[derive(Default)]
struct Viewer {
    loader: Loader,
    photo: Option<image::Handle>,
    status: String,
    capture: Option<PathBuf>,
    capture_pending: bool,
}
#[derive(Debug, Clone)]
enum Message {
    Open,
    Info(iced::system::Information),
    Picked(Option<PathBuf>),
    Poll,
    Capture,
    Captured(iced::window::Screenshot),
}
impl Viewer {
    fn new() -> Self {
        let mut app = Self {
            status: "Open a JPEG to begin · S0 experiment".into(),
            ..Self::default()
        };
        let mut args = std::env::args_os().skip(1);
        if let Some(path) = args.next() {
            app.loader.request(path.into());
            app.status = "Loading…".into();
        }
        app.capture = args.next().map(PathBuf::from);
        app
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Info(info) => eprintln!(
                "backend: {}; adapter: {}",
                info.graphics_backend, info.graphics_adapter
            ),
            Message::Open => {
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
            Message::Picked(Some(path)) => {
                self.loader.request(path);
                self.status = "Loading…".into();
            }
            Message::Picked(None) => {}
            Message::Poll => {
                if let Some(result) = self.loader.poll() {
                    match result {
                        Ok(photo) => {
                            self.status = format!(
                                "{} × {} · Fit · {:.1} ms decode",
                                photo.width, photo.height, photo.decode_ms
                            );
                            eprintln!(
                                "{}",
                                serde_json::json!({"event":"decoded","width":photo.width,"height":photo.height,"decode_ms":photo.decode_ms})
                            );
                            self.photo = Some(image::Handle::from_rgba(
                                photo.width,
                                photo.height,
                                photo.rgba,
                            ));
                            self.capture_pending = self.capture.is_some();
                        }
                        Err(error) => {
                            eprintln!("{error}");
                            self.status = error;
                        }
                    }
                }
            }
            Message::Capture => {
                self.capture_pending = false;
                return iced::window::oldest()
                    .and_then(iced::window::screenshot)
                    .map(Message::Captured);
            }
            Message::Captured(shot) => {
                if let Some(path) = self.capture.take() {
                    ::image::save_buffer(
                        &path,
                        &shot.rgba,
                        shot.size.width,
                        shot.size.height,
                        ::image::ColorType::Rgba8,
                    )
                    .expect("capture write");
                    eprintln!(
                        "{}",
                        serde_json::json!({"event":"capture","provenance":"window-renderer-readback","width":shot.size.width,"height":shot.size.height,"scale":shot.scale_factor,"color":"sRGB"})
                    );
                    return iced::exit();
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
            None => container(text("Your photograph, simply.").size(24))
                .center(Length::Fill)
                .into(),
        };
        column![
            row![
                text("Lightwell / Iced trial").size(22),
                button("Open image").on_press(Message::Open)
            ]
            .spacing(24),
            container(surface).width(Length::Fill).height(Length::Fill),
            text(&self.status).size(14)
        ]
        .spacing(16)
        .padding(24)
        .into()
    }
    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![iced::event::listen_with(|event, _, _| {
            if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key, modifiers, ..
            }) = event
                && modifiers.command()
                && matches!(key, iced::keyboard::Key::Character(ref c) if c.eq_ignore_ascii_case("o"))
            {
                return Some(Message::Open);
            }
            None
        })];
        if self.loader.loading() {
            subscriptions.push(iced::time::every(Duration::from_millis(16)).map(|_| Message::Poll));
        }
        if self.capture_pending {
            subscriptions.push(iced::window::frames().map(|_| Message::Capture));
        }
        Subscription::batch(subscriptions)
    }
}
fn main() -> iced::Result {
    iced::application(
        || {
            (
                Viewer::new(),
                iced::system::information().map(Message::Info),
            )
        },
        Viewer::update,
        Viewer::view,
    )
    .title("Lightwell · Iced probe")
    .window_size((960.0, 640.0))
    .theme(iced::Theme::Dark)
    .subscription(Viewer::subscription)
    .run()
}
