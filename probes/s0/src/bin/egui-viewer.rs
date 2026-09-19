use eframe::egui;
use lightwell_s0_probes::Loader;
use std::{path::PathBuf, time::Duration};
struct Viewer {
    loader: Loader,
    photo: Option<egui::TextureHandle>,
    status: String,
    capture: Option<PathBuf>,
    capture_pending: bool,
    picker: Option<std::sync::mpsc::Receiver<Option<PathBuf>>>,
}
impl Viewer {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        if let Some(render) = &cc.wgpu_render_state {
            eprintln!("adapter: {:?}", render.adapter.get_info());
        }
        let mut app = Self {
            loader: Loader::default(),
            photo: None,
            status: "Open a JPEG to begin · S0 experiment".into(),
            capture: None,
            capture_pending: false,
            picker: None,
        };
        let mut args = std::env::args_os().skip(1);
        if let Some(path) = args.next() {
            app.loader.request(path.into());
            app.status = "Loading…".into();
        }
        app.capture = args.next().map(PathBuf::from);
        app
    }
    fn pick(&mut self, ctx: &egui::Context) {
        if self.picker.is_some() {
            return;
        }
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .add_filter("JPEG", &["jpg", "jpeg"])
                .pick_file();
            let _ = sender.send(path);
            ctx.request_repaint();
        });
        self.picker = Some(receiver);
    }
}
impl eframe::App for Viewer {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(receiver) = &self.picker
            && let Ok(path) = receiver.try_recv()
        {
            self.picker = None;
            if let Some(path) = path {
                self.loader.request(path);
                self.status = "Loading…".into();
            }
        }
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
                    self.photo = Some(ctx.load_texture(
                        "photo",
                        egui::ColorImage::from_rgba_unmultiplied(
                            [photo.width as usize, photo.height as usize],
                            &photo.rgba,
                        ),
                        egui::TextureOptions::LINEAR,
                    ));
                    self.capture_pending = self.capture.is_some();
                }
                Err(error) => {
                    eprintln!("{error}");
                    self.status = error;
                }
            }
        }
        if self.loader.loading() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.pick(ctx);
        }
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.heading("Lightwell / egui trial");
                if ui.button("Open image").clicked() {
                    self.pick(ctx);
                }
            });
            ui.add_space(16.0);
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(&self.status);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(photo) = &self.photo {
                let available = ui.available_size();
                let size = photo.size_vec2();
                let fit = (available.x / size.x).min(available.y / size.y);
                let rect = egui::Rect::from_center_size(ui.max_rect().center(), size * fit);
                ui.put(rect, egui::Image::new(photo).fit_to_exact_size(size * fit));
            } else {
                ui.centered_and_justified(|ui| {
                    ui.heading("Your photograph, simply.");
                });
            }
        });
        if self.capture_pending {
            self.capture_pending = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        let events = ctx.input(|input| input.events.clone());
        {
            for event in &events {
                if let egui::Event::Screenshot { image, .. } = event
                    && let Some(path) = self.capture.take()
                {
                    let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    image::save_buffer(
                        &path,
                        &bytes,
                        image.width() as u32,
                        image.height() as u32,
                        image::ColorType::Rgba8,
                    )
                    .expect("capture write");
                    eprintln!(
                        "{}",
                        serde_json::json!({"event":"capture","provenance":"window-renderer-readback","width":image.width(),"height":image.height(),"scale":ctx.pixels_per_point(),"color":"sRGB"})
                    );
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }
}
fn main() -> eframe::Result {
    eframe::run_native(
        "Lightwell · egui probe",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: egui::ViewportBuilder::default().with_inner_size([960.0, 640.0]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Viewer::new(cc)))),
    )
}
