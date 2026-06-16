use std::path::PathBuf;

use eframe::egui;
use egui::{Key, KeyboardShortcut, Modifiers};

use crate::operation::{Operation, OperationGroup};
use crate::operations;

fn nice_step(min_units: f32) -> f32 {
    let min = min_units.max(1.0);
    let pow = 10f32.powf(min.log10().floor());
    for m in [1.0, 2.0, 5.0] {
        if m * pow >= min {
            return m * pow;
        }
    }
    10.0 * pow
}

fn draw_rulers(
    ui: &egui::Ui,
    full: egui::Rect,
    view: egui::Rect,
    image_min: egui::Pos2,
    zoom: f32,
) {
    if zoom <= 0.0 {
        return;
    }

    let painter = ui.painter_at(full);
    let bg = egui::Color32::from_gray(45);
    let line = egui::Color32::from_gray(120);
    let text = egui::Color32::from_gray(190);
    let font = egui::FontId::proportional(9.0);
    let step = nice_step(70.0 / zoom);

    painter.rect_filled(
        egui::Rect::from_min_max(full.min, egui::pos2(full.right(), view.top())),
        0.0,
        bg,
    );
    painter.rect_filled(
        egui::Rect::from_min_max(full.min, egui::pos2(view.left(), full.bottom())),
        0.0,
        bg,
    );

    let x_start = (view.left() - image_min.x) / zoom;
    let x_end = (view.right() - image_min.x) / zoom;
    let mut t = (x_start / step).ceil() * step;
    while t <= x_end {
        let x = image_min.x + t * zoom;
        if x >= view.left() {
            painter.line_segment(
                [egui::pos2(x, view.top() - 6.0), egui::pos2(x, view.top())],
                egui::Stroke::new(1.0, line),
            );
            painter.text(
                egui::pos2(x + 2.0, full.top() + 1.0),
                egui::Align2::LEFT_TOP,
                format!("{}", t.round() as i32),
                font.clone(),
                text,
            );
        }
        t += step;
    }

    let y_start = (view.top() - image_min.y) / zoom;
    let y_end = (view.bottom() - image_min.y) / zoom;
    let mut t = (y_start / step).ceil() * step;
    while t <= y_end {
        let y = image_min.y + t * zoom;
        if y >= view.top() {
            painter.line_segment(
                [egui::pos2(view.left() - 6.0, y), egui::pos2(view.left(), y)],
                egui::Stroke::new(1.0, line),
            );
            painter.text(
                egui::pos2(full.left() + 1.0, y + 2.0),
                egui::Align2::LEFT_TOP,
                format!("{}", t.round() as i32),
                font.clone(),
                text,
            );
        }
        t += step;
    }

    painter.rect_filled(egui::Rect::from_min_max(full.min, view.min), 0.0, bg);
}

fn operation_menu(
    ui: &mut egui::Ui,
    groups: &[OperationGroup],
    loaded: bool,
) -> Option<Box<dyn Operation>> {
    ui.style_mut().wrap = Some(false);
    let mut chosen = None;
    for (group_index, group) in groups.iter().enumerate() {
        if group_index > 0 {
            ui.separator();
        }
        ui.label(egui::RichText::new(group.label).weak().small());
        for kind in group.kinds {
            if ui
                .add_enabled(loaded, egui::Button::new(kind.menu_label))
                .clicked()
            {
                chosen = Some((kind.make)());
                ui.close_menu();
            }
        }
    }
    chosen
}

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

struct HistoryEntry {
    operation: Box<dyn Operation>,
    enabled: bool,
}

#[derive(Default)]
pub struct KiflaApp {
    original: Option<image::RgbaImage>,
    result: Option<image::RgbaImage>,
    texture: Option<egui::TextureHandle>,
    history: Vec<HistoryEntry>,
    size: [usize; 2],
    path: Option<PathBuf>,
    error: Option<String>,
    zoom: f32,
    pan: egui::Vec2,
    needs_fit: bool,
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
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                    "Kifla - {name} ({} × {})",
                    size[0], size[1]
                )));
                self.original = Some(rgba);
                self.history.clear();
                self.size = size;
                self.path = Some(path);
                self.error = None;
                self.needs_fit = true;
                self.rebuild(ctx);
            }
            Err(err) => {
                self.error = Some(format!("Failed to load image: {err}"));
            }
        }
    }

    fn rebuild(&mut self, ctx: &egui::Context) {
        let Some(original) = &self.original else {
            self.result = None;
            self.texture = None;
            return;
        };

        let mut result = original.clone();
        for entry in &self.history {
            if entry.enabled {
                entry.operation.apply(&mut result);
            }
        }

        let size = [result.width() as usize, result.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, result.as_raw());
        self.texture =
            Some(ctx.load_texture("texture", color_image, egui::TextureOptions::NEAREST));
        self.result = Some(result);
    }

    fn save(&mut self) {
        let (Some(result), Some(path)) = (&self.result, &self.path) else {
            return;
        };

        match result.save(path) {
            Ok(()) => self.error = None,
            Err(err) => self.error = Some(format!("Failed to save image: {err}")),
        }
    }

    fn save_as(&mut self, ctx: &egui::Context) {
        let Some(result) = &self.result else {
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

        match result.save(&path) {
            Ok(()) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                    "Kifla - {name} ({} × {})",
                    self.size[0], self.size[1]
                )));
                self.path = Some(path);
                self.error = None;
            }
            Err(err) => self.error = Some(format!("Failed to save image: {err}")),
        }
    }

    fn close_texture(&mut self, ctx: &egui::Context) {
        self.original = None;
        self.result = None;
        self.texture = None;
        self.history.clear();
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
        let mut add_operation: Option<Box<dyn Operation>> = None;
        let mut history_dirty = false;

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
                            egui::Button::new("📂 Open…")
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
                            egui::Button::new("💾 Save…")
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
                            egui::Button::new("💾 Save As…")
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
                            egui::Button::new("✖ Close…")
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
                            egui::Button::new("🚪 Quit…")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_QUIT)),
                        )
                        .clicked()
                    {
                        quit_requested = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Transform", |ui| {
                    if let Some(op) = operation_menu(ui, operations::TRANSFORM_GROUPS, loaded) {
                        add_operation = Some(op);
                    }
                });
                ui.menu_button("Image", |ui| {
                    if let Some(op) = operation_menu(ui, operations::OPERATION_GROUPS, loaded) {
                        add_operation = Some(op);
                    }
                });
            });
        });

        if self.original.is_some() && !self.history.is_empty() {
            egui::SidePanel::left("history_panel")
                .resizable(true)
                .default_width(240.0)
                .show(ctx, |ui| {
                    ui.heading("Edits");
                    ui.separator();

                    let mut remove_index = None;
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (i, entry) in self.history.iter_mut().enumerate() {
                                if entry.operation.has_settings() {
                                    let id = egui::Id::new(("history_entry", i));
                                    let state =
                                egui::collapsing_header::CollapsingState::load_with_default_open(
                                    ui.ctx(),
                                    id,
                                    false,
                                );
                                    let header = state.show_header(ui, |ui| {
                                        if ui.checkbox(&mut entry.enabled, "").changed() {
                                            history_dirty = true;
                                        }
                                        let enabled = entry.enabled;
                                        ui.add_enabled_ui(enabled, |ui| {
                                            ui.label(entry.operation.name());
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.small_button("×").clicked() {
                                                    remove_index = Some(i);
                                                }
                                            },
                                        );
                                    });
                                    let enabled = entry.enabled;
                                    header.body(|ui| {
                                        ui.add_enabled_ui(enabled, |ui| {
                                            if entry.operation.settings_ui(ui) {
                                                history_dirty = true;
                                            }
                                        });
                                    });
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.add_space(ui.spacing().indent);
                                        if ui.checkbox(&mut entry.enabled, "").changed() {
                                            history_dirty = true;
                                        }
                                        let enabled = entry.enabled;
                                        ui.add_enabled_ui(enabled, |ui| {
                                            ui.label(entry.operation.name());
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.small_button("×").clicked() {
                                                    remove_index = Some(i);
                                                }
                                            },
                                        );
                                    });
                                }
                            }
                        });

                    if let Some(i) = remove_index {
                        self.history.remove(i);
                        history_dirty = true;
                    }
                });
        }

        let canvas_frame =
            egui::Frame::central_panel(&ctx.style()).fill(egui::Color32::from_gray(28));
        egui::CentralPanel::default()
            .frame(canvas_frame)
            .show(ctx, |ui| {
                if let Some(error) = &self.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }

                let Some(texture) = self.texture.clone() else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() / 2.0 - 30.0);
                        ui.label(egui::RichText::new("Open a texture to get started...").weak());
                        ui.add_space(8.0);
                        ui.scope(|ui| {
                            ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
                            if ui.button("📂 Open…").clicked() {
                                open_requested = true;
                            }
                        });
                    });
                    return;
                };

                let full = ui.available_rect_before_wrap();
                let ruler = 20.0;
                let rect = egui::Rect::from_min_max(
                    egui::pos2(full.left() + ruler, full.top() + ruler),
                    full.max,
                );
                let response = ui.allocate_rect(rect, egui::Sense::drag());
                let tex_size = texture.size_vec2();

                if self.needs_fit {
                    self.zoom = (rect.width() / tex_size.x).min(rect.height() / tex_size.y);
                    self.pan = egui::Vec2::ZERO;
                    self.needs_fit = false;
                }

                if response.dragged() {
                    self.pan += response.drag_delta();
                }

                if response.hovered() {
                    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                    if scroll != 0.0 {
                        let new_zoom = (self.zoom * (scroll * 0.0015).exp()).clamp(0.05, 64.0);
                        if let Some(cursor) = response.hover_pos() {
                            let to_cursor = cursor - rect.center();
                            let factor = new_zoom / self.zoom;
                            self.pan = to_cursor - (to_cursor - self.pan) * factor;
                        }
                        self.zoom = new_zoom;
                    }
                }

                if response.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                } else if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }

                let image_rect =
                    egui::Rect::from_center_size(rect.center() + self.pan, tex_size * self.zoom);
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                ui.painter_at(rect)
                    .image(texture.id(), image_rect, uv, egui::Color32::WHITE);

                if let Some(cursor) = response.hover_pos() {
                    let painter = ui.painter_at(rect);
                    let guide = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(90));
                    painter.line_segment(
                        [
                            egui::pos2(cursor.x, rect.top()),
                            egui::pos2(cursor.x, rect.bottom()),
                        ],
                        guide,
                    );
                    painter.line_segment(
                        [
                            egui::pos2(rect.left(), cursor.y),
                            egui::pos2(rect.right(), cursor.y),
                        ],
                        guide,
                    );
                }

                draw_rulers(ui, full, rect, image_rect.min, self.zoom);
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
        if let Some(mut operation) = add_operation {
            if let Some(result) = &self.result {
                operation.on_added(result.width(), result.height());
            }
            let has_settings = operation.has_settings();
            self.history.push(HistoryEntry {
                operation,
                enabled: true,
            });
            if has_settings {
                let id = egui::Id::new(("history_entry", self.history.len() - 1));
                let mut state =
                    egui::collapsing_header::CollapsingState::load_with_default_open(ctx, id, true);
                state.set_open(true);
                state.store(ctx);
            }
            history_dirty = true;
        }
        if history_dirty {
            self.rebuild(ctx);
        }
    }
}
