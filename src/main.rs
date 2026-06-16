use std::path::PathBuf;

use eframe::egui;
use egui::{Key, KeyboardShortcut, Modifiers};

const SHORTCUT_OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::O);
const SHORTCUT_SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::S);
const SHORTCUT_SAVE_AS: KeyboardShortcut = KeyboardShortcut::new(
    Modifiers {
        alt: false,
        ctrl: true,
        shift: true,
        mac_cmd: false,
        command: false,
    },
    Key::S,
);
const SHORTCUT_CLOSE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::W);
const SHORTCUT_QUIT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Q);

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("Kifla"),
        ..Default::default()
    };

    eframe::run_native(
        "Kifla",
        options,
        Box::new(|_cc| Box::<KiflaApp>::default()),
    )
}

#[derive(Default)]
struct KiflaApp {
    image: Option<image::RgbaImage>,
    texture: Option<egui::TextureHandle>,
    size: [usize; 2],
    path: Option<PathBuf>,
    error: Option<String>,
}

impl KiflaApp {
    fn open_texture(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Images",
                &["png", "jpg", "jpeg", "bmp", "tga", "tiff", "webp"],
            )
            .pick_file()
        else {
            return;
        };

        match image::open(&path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                let handle =
                    ctx.load_texture("texture", color_image, egui::TextureOptions::default());
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                    "Kifla — {name} ({} × {})",
                    size[0], size[1]
                )));
                self.image = Some(rgba);
                self.texture = Some(handle);
                self.size = size;
                self.path = Some(path);
                self.error = None;
            }
            Err(err) => {
                self.error = Some(format!("Failed to load image: {err}"));
            }
        }
    }

    fn save(&mut self) {
        let (Some(image), Some(path)) = (&self.image, &self.path) else {
            return;
        };

        match image.save(path) {
            Ok(()) => self.error = None,
            Err(err) => self.error = Some(format!("Failed to save image: {err}")),
        }
    }

    fn save_as(&mut self, ctx: &egui::Context) {
        let Some(image) = &self.image else {
            return;
        };

        let mut dialog = rfd::FileDialog::new()
            .add_filter("PNG", &["png"])
            .add_filter("JPEG", &["jpg", "jpeg"])
            .add_filter("Bitmap", &["bmp"])
            .add_filter("TGA", &["tga"]);
        if let Some(path) = &self.path {
            if let Some(name) = path.file_name() {
                dialog = dialog.set_file_name(name.to_string_lossy());
            }
            if let Some(dir) = path.parent() {
                dialog = dialog.set_directory(dir);
            }
        }

        let Some(path) = dialog.save_file() else {
            return;
        };

        match image.save(&path) {
            Ok(()) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                    "Kifla — {name} ({} × {})",
                    self.size[0], self.size[1]
                )));
                self.path = Some(path);
                self.error = None;
            }
            Err(err) => self.error = Some(format!("Failed to save image: {err}")),
        }
    }

    fn close_texture(&mut self, ctx: &egui::Context) {
        self.image = None;
        self.texture = None;
        self.size = [0, 0];
        self.path = None;
        self.error = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Title("Kifla".to_owned()));
    }
}

impl eframe::App for KiflaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let loaded = self.texture.is_some();
        let mut open_requested = false;
        let mut save_requested = false;
        let mut save_as_requested = false;
        let mut close_requested = false;
        let mut quit_requested = false;

        ctx.input_mut(|i| {
            open_requested |= i.consume_shortcut(&SHORTCUT_OPEN);
            quit_requested |= i.consume_shortcut(&SHORTCUT_QUIT);
            if loaded {
                save_requested |= i.consume_shortcut(&SHORTCUT_SAVE);
                save_as_requested |= i.consume_shortcut(&SHORTCUT_SAVE_AS);
                close_requested |= i.consume_shortcut(&SHORTCUT_CLOSE);
            }
        });

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add(
                            egui::Button::new("Open…")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_OPEN)),
                        )
                        .clicked()
                    {
                        open_requested = true;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            loaded,
                            egui::Button::new("Save")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_SAVE)),
                        )
                        .clicked()
                    {
                        save_requested = true;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            loaded,
                            egui::Button::new("Save As…")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_SAVE_AS)),
                        )
                        .clicked()
                    {
                        save_as_requested = true;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            loaded,
                            egui::Button::new("Close")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_CLOSE)),
                        )
                        .clicked()
                    {
                        close_requested = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add(
                            egui::Button::new("Quit")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_QUIT)),
                        )
                        .clicked()
                    {
                        quit_requested = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Tools", |_ui| {});
            });
        });

        egui::SidePanel::left("tool_settings_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Tool Settings");
                ui.separator();
                ui.label("No tool selected.");
            });

        egui::SidePanel::right("history_panel")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("History");
                ui.separator();
                if self.texture.is_none() {
                    ui.label("No texture loaded.");
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }

            match &self.texture {
                Some(texture) => {
                    let available = ui.available_size();
                    let tex_size = texture.size_vec2();
                    let scale = (available.x / tex_size.x).min(available.y / tex_size.y);
                    let display = tex_size * scale;
                    ui.centered_and_justified(|ui| {
                        ui.add(egui::Image::new(egui::load::SizedTexture::new(
                            texture.id(),
                            display,
                        )));
                    });
                }
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.label("Open a texture to get started (File → Open…).");
                    });
                }
            }
        });

        if open_requested {
            self.open_texture(ctx);
        }
        if save_requested {
            self.save();
        }
        if save_as_requested {
            self.save_as(ctx);
        }
        if close_requested {
            self.close_texture(ctx);
        }
        if quit_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
