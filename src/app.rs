use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};

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
    let minor_line = line;
    let text = egui::Color32::from_gray(190);
    let font = egui::FontId::proportional(9.0);
    let step = nice_step(70.0 / zoom);

    let lead = step / 10f32.powf(step.log10().floor());
    let subs = if lead < 1.5 {
        10
    } else if lead < 3.0 {
        8
    } else {
        10
    };
    let minor = step / subs as f32;

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

    let mut tm = (x_start / minor).ceil() * minor;
    while tm <= x_end {
        let x = image_min.x + tm * zoom;
        if x >= view.left() {
            painter.line_segment(
                [egui::pos2(x, view.top() - 3.0), egui::pos2(x, view.top())],
                egui::Stroke::new(1.0, minor_line),
            );
        }
        tm += minor;
    }

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

    let mut tm = (y_start / minor).ceil() * minor;
    while tm <= y_end {
        let y = image_min.y + tm * zoom;
        if y >= view.top() {
            painter.line_segment(
                [egui::pos2(view.left() - 3.0, y), egui::pos2(view.left(), y)],
                egui::Stroke::new(1.0, minor_line),
            );
        }
        tm += minor;
    }

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

fn save_image(image: &image::RgbaImage, path: &std::path::Path) -> image::ImageResult<()> {
    let is_ico = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ico"));
    if !is_ico {
        return image.save(path);
    }

    let max_dim = image.width().max(image.height()).max(1);
    let mut sizes: Vec<u32> = [256, 128, 64, 48, 32, 16]
        .into_iter()
        .filter(|&n| n <= max_dim)
        .collect();
    if sizes.is_empty() {
        sizes.push(max_dim.min(256));
    }

    let mut frames = Vec::new();
    for size in sizes {
        let resized =
            image::imageops::resize(image, size, size, image::imageops::FilterType::Lanczos3);
        frames.push(image::codecs::ico::IcoFrame::as_png(
            resized.as_raw(),
            size,
            size,
            image::ColorType::Rgba8,
        )?);
    }

    let file = std::fs::File::create(path)?;
    image::codecs::ico::IcoEncoder::new(file).encode_images(&frames)
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

fn add_operation_matches(filter: &str) -> usize {
    let needle = filter.trim().to_lowercase();
    operations::TRANSFORM_GROUPS
        .iter()
        .chain(operations::OPERATION_GROUPS.iter())
        .flat_map(|g| g.kinds.iter())
        .filter(|k| needle.is_empty() || k.menu_label.to_lowercase().contains(&needle))
        .count()
}

fn add_operation_list(
    ui: &mut egui::Ui,
    loaded: bool,
    filter: &str,
    selected: usize,
    activate: bool,
    scroll_to_selected: bool,
) -> Option<Box<dyn Operation>> {
    ui.style_mut().wrap = Some(false);
    let needle = filter.trim().to_lowercase();
    let mut chosen = None;
    let mut first = true;
    let mut flat = 0usize;
    let highlight = ui.visuals().widgets.hovered.weak_bg_fill;
    let groups = operations::TRANSFORM_GROUPS
        .iter()
        .chain(operations::OPERATION_GROUPS.iter());
    for group in groups {
        let matches: Vec<_> = group
            .kinds
            .iter()
            .filter(|k| needle.is_empty() || k.menu_label.to_lowercase().contains(&needle))
            .collect();
        if matches.is_empty() {
            continue;
        }
        if !first {
            ui.separator();
        }
        first = false;
        ui.label(egui::RichText::new(group.label).weak().small());
        for kind in matches {
            let is_sel = flat == selected;
            let fill = if is_sel {
                highlight
            } else {
                egui::Color32::TRANSPARENT
            };
            let resp = ui.add_enabled(
                loaded,
                egui::Button::new(kind.menu_label)
                    .fill(fill)
                    .min_size(egui::vec2(ui.available_width(), 0.0)),
            );
            if resp.clicked() || (is_sel && activate) {
                chosen = Some((kind.make)());
            }
            if is_sel && scroll_to_selected {
                resp.scroll_to_me(Some(egui::Align::Center));
            }
            flat += 1;
        }
    }
    if flat == 0 {
        ui.label(egui::RichText::new("No matches").weak());
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
const SHORTCUT_FIT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Num0);
const SHORTCUT_ADD: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::A);
const COMPARE_KEY: Key = Key::C;
const SHORTCUT_COMPARE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, COMPARE_KEY);

struct EditEntry {
    id: u64,
    operation: Box<dyn Operation>,
    enabled: bool,
}

#[derive(Default)]
pub struct KiflaApp {
    original: Option<image::RgbaImage>,
    result: Option<image::RgbaImage>,
    texture: Option<egui::TextureHandle>,
    original_texture: Option<egui::TextureHandle>,
    edits: Vec<EditEntry>,
    size: [usize; 2],
    path: Option<PathBuf>,
    error: Option<String>,
    zoom: f32,
    pan: egui::Vec2,
    fit: bool,
    view: Option<(egui::Rect, egui::Pos2)>,
    dragging: Option<usize>,
    drag_grab: f32,
    reordered: bool,
    row_heights: std::collections::HashMap<u64, f32>,
    next_id: u64,
    dirty: bool,
    add_filter: String,
    add_selected: usize,
    add_was_open: bool,
    last_apply: f64,
    pending_open: Option<Receiver<Option<PathBuf>>>,
    pending_save: Option<Receiver<Option<PathBuf>>>,
}

impl KiflaApp {
    fn open_texture(&mut self) {
        if self.pending_open.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .add_filter(
                    "Images",
                    &["png", "jpg", "jpeg", "bmp", "tga", "tiff", "webp"],
                )
                .pick_file();
            let _ = tx.send(path);
        });
        self.pending_open = Some(rx);
    }

    fn load_image(&mut self, path: PathBuf, ctx: &egui::Context) {
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
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                self.original_texture = Some(ctx.load_texture(
                    "original",
                    color_image,
                    egui::TextureOptions::NEAREST,
                ));
                self.original = Some(rgba);
                self.edits.clear();
                self.size = size;
                self.path = Some(path);
                self.error = None;
                self.fit = true;
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
        for entry in &self.edits {
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

        match save_image(result, path) {
            Ok(()) => self.error = None,
            Err(err) => self.error = Some(format!("Failed to save image: {err}")),
        }
    }

    fn save_as(&mut self) {
        if self.result.is_none() || self.pending_save.is_some() {
            return;
        }

        let mut dialog = rfd::FileDialog::new()
            .add_filter("PNG", &["png"])
            .add_filter("JPEG", &["jpg", "jpeg"])
            .add_filter("Bitmap", &["bmp"])
            .add_filter("TGA", &["tga"])
            .add_filter("Icon", &["ico"]);
        if let Some(path) = &self.path {
            if let Some(name) = path.file_name() {
                dialog = dialog.set_file_name(name.to_string_lossy().into_owned());
            }
            if let Some(dir) = path.parent() {
                dialog = dialog.set_directory(dir);
            }
        }

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(dialog.save_file());
        });
        self.pending_save = Some(rx);
    }

    fn write_result(&mut self, path: PathBuf, ctx: &egui::Context) {
        let Some(result) = &self.result else {
            return;
        };
        match save_image(result, &path) {
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
        self.original_texture = None;
        self.edits.clear();
        self.size = [0, 0];
        self.path = None;
        self.error = None;
        self.view = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Title("Kifla".to_owned()));
    }

    fn add_button(
        &mut self,
        ui: &mut egui::Ui,
        loaded: bool,
        add_requested: bool,
    ) -> Option<Box<dyn Operation>> {
        let mut result = None;
        ui.add_space(2.0);
        ui.vertical_centered(|ui| {
            let tip = format!("New edit  ({})", ui.ctx().format_shortcut(&SHORTCUT_ADD));
            let add = ui
                .add(egui::Button::new("➕ Add…").min_size(egui::vec2(132.0, 0.0)))
                .on_hover_ui_at_pointer(|ui| {
                    ui.label(
                        egui::RichText::new(tip)
                            .size(11.0)
                            .color(egui::Color32::from_gray(190)),
                    );
                });
            let popup_id = ui.make_persistent_id("add_popup");
            if add.clicked() {
                ui.memory_mut(|m| m.toggle_popup(popup_id));
            }
            if add_requested {
                ui.memory_mut(|m| m.open_popup(popup_id));
            }
            let open = ui.memory(|m| m.is_popup_open(popup_id));
            let just_opened = open && !self.add_was_open;
            self.add_was_open = open;
            if just_opened {
                self.add_filter.clear();
                self.add_selected = 0;
            }
            let dir = if add.rect.center().y > ui.ctx().screen_rect().center().y {
                egui::AboveOrBelow::Above
            } else {
                egui::AboveOrBelow::Below
            };

            let (mut nav_up, mut nav_down, mut activate) = (false, false, false);
            if open {
                ui.input_mut(|i| {
                    nav_up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
                    nav_down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
                    activate = i.consume_key(Modifiers::NONE, Key::Enter);
                });
            }
            let count = add_operation_matches(&self.add_filter);
            if count > 0 {
                if nav_down {
                    self.add_selected = (self.add_selected + 1) % count;
                }
                if nav_up {
                    self.add_selected = (self.add_selected + count - 1) % count;
                }
                self.add_selected = self.add_selected.min(count - 1);
            } else {
                self.add_selected = 0;
            }
            let scroll_to_sel = nav_up || nav_down;
            let selected = self.add_selected;

            let filter = &mut self.add_filter;
            let chosen = egui::popup_above_or_below_widget(ui, popup_id, &add, dir, |ui| {
                ui.set_min_width(190.0);
                let search = ui
                    .scope(|ui| {
                        let edge = egui::Color32::from_gray(105);
                        let v = ui.visuals_mut();
                        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, edge);
                        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, edge);
                        v.widgets.active.bg_stroke = egui::Stroke::new(1.0, edge);
                        ui.add(
                            egui::TextEdit::singleline(filter)
                                .hint_text(
                                    egui::RichText::new("Search…")
                                        .color(egui::Color32::from_gray(130)),
                                )
                                .desired_width(f32::INFINITY),
                        )
                    })
                    .inner;
                if just_opened {
                    search.request_focus();
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .show(ui, |ui| {
                        add_operation_list(ui, loaded, filter, selected, activate, scroll_to_sel)
                    })
                    .inner
            });
            if let Some(op) = chosen.flatten() {
                result = Some(op);
                ui.memory_mut(|m| m.close_popup());
            }
        });
        result
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
        let mut edits_dirty = false;

        let mut fit_requested = false;
        let mut add_requested = false;
        let typing = ctx.memory(|m| m.focus().is_some());
        ctx.input_mut(|i| {
            open_requested |= i.consume_shortcut(&SHORTCUT_OPEN);
            quit_requested |= i.consume_shortcut(&SHORTCUT_QUIT);
            if loaded {
                save_requested |= i.consume_shortcut(&SHORTCUT_SAVE);
                save_as_requested |= i.consume_shortcut(&SHORTCUT_SAVE_AS);
                close_requested |= i.consume_shortcut(&SHORTCUT_CLOSE);
                fit_requested |= i.consume_shortcut(&SHORTCUT_FIT);
                if !typing {
                    add_requested |= i.consume_shortcut(&SHORTCUT_ADD);
                }
            }
        });
        let mut compare_held = loaded && ctx.input(|i| i.key_down(COMPARE_KEY));

        let dropped = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .rev()
                .find_map(|f| f.path.clone())
        });
        if let Some(path) = dropped {
            self.load_image(path, ctx);
        }

        match self.pending_open.as_ref().map(Receiver::try_recv) {
            Some(Ok(result)) => {
                self.pending_open = None;
                if let Some(path) = result {
                    self.load_image(path, ctx);
                }
            }
            Some(Err(TryRecvError::Empty)) => ctx.request_repaint(),
            Some(Err(TryRecvError::Disconnected)) => self.pending_open = None,
            None => {}
        }
        match self.pending_save.as_ref().map(Receiver::try_recv) {
            Some(Ok(result)) => {
                self.pending_save = None;
                if let Some(path) = result {
                    self.write_result(path, ctx);
                }
            }
            Some(Err(TryRecvError::Empty)) => ctx.request_repaint(),
            Some(Err(TryRecvError::Disconnected)) => self.pending_save = None,
            None => {}
        }

        let loaded = self.texture.is_some();

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
                ui.menu_button("View", |ui| {
                    if ui
                        .add_enabled(
                            loaded,
                            egui::Button::new("Fit")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_FIT)),
                        )
                        .clicked()
                    {
                        fit_requested = true;
                        ui.close_menu();
                    }
                    let original = ui.add_enabled(
                        loaded,
                        egui::Button::new("Show Original")
                            .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_COMPARE)),
                    );
                    if original.is_pointer_button_down_on() {
                        compare_held = true;
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

        if fit_requested {
            self.fit = true;
        }
        let comparing = compare_held && self.original_texture.is_some();

        if loaded {
            const EDITS_WIDTH: f32 = 258.0;
            egui::SidePanel::left("edits_panel")
                .resizable(true)
                .default_width(EDITS_WIDTH)
                .width_range(80.0..=EDITS_WIDTH)
                .show(ctx, |ui| {
                    ui.spacing_mut().slider_width = 150.0;
                    ui.style_mut().spacing.scroll.floating = false;
                    ui.style_mut().spacing.scroll.bar_width = 5.0;

                    ui.heading(
                        egui::RichText::new("Edit Stack").color(egui::Color32::from_gray(120)),
                    );
                    ui.separator();

                    let mut remove_index = None;
                    let mut set_collapse: Option<(u64, bool)> = None;
                    let mut drag_start: Option<usize> = None;
                    let mut reorder: Option<(usize, usize)> = None;
                    let mut new_grab: Option<f32> = None;
                    let mut measured: Vec<(u64, f32)> = Vec::new();
                    let dragging = self.dragging;
                    let drag_grab = self.drag_grab;
                    let spacing = ui.spacing().item_spacing.y;

                    let heights: Vec<f32> = self
                        .edits
                        .iter()
                        .map(|e| self.row_heights.get(&e.id).copied().unwrap_or(24.0))
                        .collect();
                    let mut slots = Vec::with_capacity(self.edits.len());
                    let mut total = 0.0;
                    for h in &heights {
                        slots.push(total);
                        total += h + spacing;
                    }

                    let pin_add = total + 36.0 > ui.available_height();
                    if pin_add {
                        let op = egui::TopBottomPanel::bottom("edits_add_bar")
                            .show_inside(ui, |ui| self.add_button(ui, loaded, add_requested))
                            .inner;
                        if op.is_some() {
                            add_operation = op;
                        }
                    }

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let width = ui.available_width();
                            let (area, _) = ui.allocate_exact_size(
                                egui::vec2(width, total.max(1.0)),
                                egui::Sense::hover(),
                            );
                            let origin = area.min;
                            let pointer = ui.input(|i| i.pointer.hover_pos());

                            let eye_toggle = |ui: &mut egui::Ui, entry: &mut EditEntry| {
                                let text = if entry.enabled {
                                    egui::RichText::new("👁")
                                } else {
                                    egui::RichText::new("👁").weak()
                                };
                                if ui.selectable_label(false, text).clicked() {
                                    entry.enabled = !entry.enabled;
                                    true
                                } else {
                                    false
                                }
                            };

                            let name_handle = |ui: &mut egui::Ui, entry: &EditEntry, dim: bool| {
                                let size =
                                    egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
                                let resp = ui.allocate_response(size, egui::Sense::drag());
                                let color = if dim {
                                    ui.visuals().weak_text_color()
                                } else {
                                    egui::Color32::from_gray(175)
                                };
                                ui.painter().text(
                                    resp.rect.left_center() + egui::vec2(1.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    entry.operation.name(),
                                    egui::FontId::proportional(14.0),
                                    color,
                                );
                                if resp.hovered() || resp.dragged() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                }
                                resp.drag_started()
                            };

                            let order: Vec<usize> = (0..self.edits.len())
                                .filter(|i| Some(*i) != dragging)
                                .chain(dragging)
                                .collect();
                            for i in order {
                                let entry = &mut self.edits[i];
                                let is_dragged = dragging == Some(i);
                                let target = if is_dragged {
                                    match pointer {
                                        Some(p) => (p.y - origin.y) - drag_grab,
                                        None => slots[i],
                                    }
                                } else {
                                    slots[i]
                                };
                                let dur = if is_dragged { 0.0 } else { 0.12 };
                                let y = ui.ctx().animate_value_with_time(
                                    egui::Id::new(("row_y", entry.id)),
                                    target,
                                    dur,
                                );
                                let row_rect = egui::Rect::from_min_size(
                                    egui::pos2(origin.x, (origin.y + y).round()),
                                    egui::vec2(width, heights[i].max(1.0)),
                                );
                                let dim = !entry.enabled;

                                let inner = ui.allocate_ui_at_rect(row_rect, |ui| {
                                    if entry.operation.has_settings() {
                                        let cid = egui::Id::new(("edit_body", entry.id));
                                        egui::collapsing_header::CollapsingState::load_with_default_open(
                                            ui.ctx(),
                                            cid,
                                            false,
                                        )
                                        .show_header(ui, |ui| {
                                            ui.spacing_mut().item_spacing.x = 2.0;
                                            if eye_toggle(ui, entry) {
                                                edits_dirty = true;
                                                set_collapse = Some((entry.id, entry.enabled));
                                            }
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.small_button("×").clicked() {
                                                        remove_index = Some(i);
                                                    }
                                                    if name_handle(ui, entry, dim) {
                                                        drag_start = Some(i);
                                                    }
                                                },
                                            );
                                        })
                                        .body(|ui| {
                                            let enabled = entry.enabled;
                                            ui.add_enabled_ui(enabled, |ui| {
                                                if entry.operation.settings_ui(ui) {
                                                    edits_dirty = true;
                                                }
                                            });
                                        });
                                    } else {
                                        ui.horizontal(|ui| {
                                            ui.add_space(ui.spacing().indent);
                                            ui.spacing_mut().item_spacing.x = 2.0;
                                            if eye_toggle(ui, entry) {
                                                edits_dirty = true;
                                                set_collapse = Some((entry.id, entry.enabled));
                                            }
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.small_button("×").clicked() {
                                                        remove_index = Some(i);
                                                    }
                                                    if name_handle(ui, entry, dim) {
                                                        drag_start = Some(i);
                                                    }
                                                },
                                            );
                                        });
                                    }
                                    ui.separator();
                                });

                                let used_h = inner.response.rect.height();
                                measured.push((entry.id, used_h.round()));
                                if is_dragged {
                                    let band = egui::Rect::from_min_size(
                                        egui::pos2(origin.x, (origin.y + y).round()),
                                        egui::vec2(width, used_h),
                                    );
                                    ui.painter().rect_filled(
                                        band,
                                        3.0,
                                        egui::Color32::from_white_alpha(5),
                                    );
                                }
                            }

                            if let (Some(i), Some(p)) = (drag_start, pointer) {
                                new_grab = Some((p.y - origin.y) - slots[i]);
                            }
                            if let (Some(d), Some(p)) = (dragging, pointer) {
                                let center = (p.y - origin.y) - drag_grab;
                                let mut new_index = 0;
                                for (j, sy) in slots.iter().enumerate() {
                                    if j != d && sy + heights[j] / 2.0 < center {
                                        new_index += 1;
                                    }
                                }
                                if new_index != d {
                                    reorder = Some((d, new_index));
                                }
                            }

                            if !pin_add {
                                let add_rect = egui::Rect::from_min_size(
                                    egui::pos2(origin.x, origin.y + total),
                                    egui::vec2(width, 0.0),
                                );
                                let op = ui
                                    .allocate_ui_at_rect(add_rect, |ui| {
                                        self.add_button(ui, loaded, add_requested)
                                    })
                                    .inner;
                                if let Some(op) = op {
                                    add_operation = Some(op);
                                }
                            }
                        });

                    for (id, h) in measured {
                        self.row_heights.insert(id, h);
                    }
                    if let Some((id, open)) = set_collapse {
                        let cid = egui::Id::new(("edit_body", id));
                        let mut state =
                            egui::collapsing_header::CollapsingState::load_with_default_open(
                                ctx, cid, false,
                            );
                        state.set_open(open);
                        state.store(ctx);
                    }
                    if let Some(i) = drag_start {
                        self.dragging = Some(i);
                        if let Some(g) = new_grab {
                            self.drag_grab = g;
                        }
                    }
                    if let Some((from, to)) = reorder {
                        let entry = self.edits.remove(from);
                        self.edits.insert(to, entry);
                        self.dragging = Some(to);
                        self.reordered = true;
                    }
                    if !ctx.input(|i| i.pointer.primary_down()) {
                        self.dragging = None;
                        if self.reordered {
                            self.reordered = false;
                            edits_dirty = true;
                        }
                    }
                    if let Some(i) = remove_index {
                        self.edits.remove(i);
                        self.dragging = None;
                        edits_dirty = true;
                    }
                });
        }

        if loaded {
            egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let mut coords = String::new();
                    let sample = if comparing { &self.original } else { &self.result };
                    if let (Some((view, origin)), Some(image)) = (self.view, sample) {
                        if let Some(p) = ctx.input(|i| i.pointer.hover_pos()) {
                            if view.contains(p) {
                                let x = ((p.x - origin.x) / self.zoom).floor() as i32;
                                let y = ((p.y - origin.y) / self.zoom).floor() as i32;
                                if x >= 0
                                    && y >= 0
                                    && (x as u32) < image.width()
                                    && (y as u32) < image.height()
                                {
                                    let px = image.get_pixel(x as u32, y as u32);
                                    coords = format!(
                                        "{x}, {y}    rgba({}, {}, {}, {})",
                                        px[0], px[1], px[2], px[3]
                                    );
                                }
                            }
                        }
                    }
                    let zoom_pct = match (self.view, &self.result) {
                        (Some((view, _)), Some(image)) => {
                            let fit = (view.width() / image.width() as f32)
                                .min(view.height() / image.height() as f32);
                            if fit > 0.0 {
                                self.zoom / fit * 100.0
                            } else {
                                100.0
                            }
                        }
                        _ => 100.0,
                    };
                    ui.label(coords);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if comparing {
                            ui.label("original");
                            ui.separator();
                        }
                        ui.label(format!("{zoom_pct:.0}%"));
                    });
                });
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
                    let area = ui.max_rect();
                    ui.painter().text(
                        egui::pos2(area.center().x, area.top() + 40.0),
                        egui::Align2::CENTER_TOP,
                        "Kifla",
                        egui::FontId::proportional(112.0),
                        egui::Color32::from_gray(60),
                    );
                    ui.vertical_centered(|ui| {
                        ui.add_space((ui.available_height() / 2.0 - 107.0).max(0.0));
                        ui.label(
                            egui::RichText::new("Open a texture to get started…")
                                .weak()
                                .size(14.0),
                        );
                        ui.add_space(8.0);
                        ui.scope(|ui| {
                            ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
                            let tip = format!(
                                "Open a texture  ({})",
                                ui.ctx().format_shortcut(&SHORTCUT_OPEN)
                            );
                            let open = ui.button("📂 Open…").on_hover_ui_at_pointer(|ui| {
                                ui.label(
                                    egui::RichText::new(tip)
                                        .size(11.0)
                                        .color(egui::Color32::from_gray(190)),
                                );
                            });
                            if open.clicked() {
                                open_requested = true;
                            }
                        });
                        ui.add_space(14.0);
                        egui::Frame::none()
                            .stroke(egui::Stroke::new(1.0, ui.visuals().weak_text_color()))
                            .rounding(6.0)
                            .inner_margin(egui::Margin::same(10.0))
                            .show(ui, |ui| {
                                ui.set_width(250.0);
                                ui.set_height(50.0);
                                ui.centered_and_justified(|ui| {
                                    ui.label(
                                        egui::RichText::new("…or drop a texture here…")
                                            .weak()
                                            .size(14.0),
                                    );
                                });
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

                if self.fit {
                    self.zoom = (rect.width() / tex_size.x).min(rect.height() / tex_size.y);
                    self.pan = egui::Vec2::ZERO;
                }

                if response.dragged() {
                    self.pan += response.drag_delta();
                    self.fit = false;
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
                        self.fit = false;
                    }
                }

                if response.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                } else if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }

                let image_rect =
                    egui::Rect::from_center_size(rect.center() + self.pan, tex_size * self.zoom);
                self.view = Some((rect, image_rect.min));
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                let draw_id = match (comparing, &self.original_texture) {
                    (true, Some(original)) => original.id(),
                    _ => texture.id(),
                };
                ui.painter_at(rect)
                    .image(draw_id, image_rect, uv, egui::Color32::WHITE);

                if let Some(cursor) = response.hover_pos() {
                    let painter = ui.painter_at(rect);
                    let guide =
                        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 128));
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
            self.open_texture();
        }
        if save_requested {
            self.save();
        }
        if save_as_requested {
            self.save_as();
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
            self.next_id += 1;
            let entry_id = self.next_id;
            self.edits.push(EditEntry {
                id: entry_id,
                operation,
                enabled: true,
            });
            if has_settings {
                let id = egui::Id::new(("edit_body", entry_id));
                let mut state =
                    egui::collapsing_header::CollapsingState::load_with_default_open(ctx, id, true);
                state.set_open(true);
                state.store(ctx);
            }
            edits_dirty = true;
        }
        self.dirty |= edits_dirty;
        if self.dirty {
            const INTERVAL: f64 = 1.0 / 30.0;
            let now = ctx.input(|i| i.time);
            let elapsed = now - self.last_apply;
            if elapsed >= INTERVAL {
                self.rebuild(ctx);
                self.last_apply = now;
                self.dirty = false;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(INTERVAL - elapsed));
            }
        }
    }
}
