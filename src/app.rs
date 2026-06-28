use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};

use eframe::egui;
use egui::{Key, KeyboardShortcut, Modifiers};

use crate::gpu::GpuContext;
use crate::modifier::{Modifier, ModifierGroup};
use crate::modifiers;

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

    // `step` is always 1, 2, or 5 times a power of ten; the leading digit picks
    // how many minor ticks subdivide it (2 splits into 8, 1 and 5 into 10).
    let lead = step / 10f32.powf(step.log10().floor());
    let subs = if (1.5..3.0).contains(&lead) { 8 } else { 10 };
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
                egui::Stroke::new(1.0, line),
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
                egui::Stroke::new(1.0, line),
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

fn modifier_menu(
    ui: &mut egui::Ui,
    groups: &[ModifierGroup],
    loaded: bool,
) -> Option<Box<dyn Modifier>> {
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

fn add_modifier_matches(filter: &str) -> usize {
    let needle = filter.trim().to_lowercase();
    modifiers::TRANSFORM_GROUPS
        .iter()
        .chain(modifiers::IMAGE_GROUPS.iter())
        .flat_map(|g| g.kinds.iter())
        .filter(|k| needle.is_empty() || k.menu_label.to_lowercase().contains(&needle))
        .count()
}

fn add_modifier_list(
    ui: &mut egui::Ui,
    loaded: bool,
    filter: &str,
    selected: usize,
    activate: bool,
    scroll_to_selected: bool,
) -> Option<Box<dyn Modifier>> {
    ui.style_mut().wrap = Some(false);
    let needle = filter.trim().to_lowercase();
    let mut chosen = None;
    let mut first = true;
    let mut flat = 0usize;
    let highlight = ui.visuals().widgets.hovered.weak_bg_fill;
    let groups = modifiers::TRANSFORM_GROUPS
        .iter()
        .chain(modifiers::IMAGE_GROUPS.iter());
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
const SHORTCUT_RECENTER: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::R);
const SHORTCUT_ADD: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::A);
const SHORTCUT_ABOUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F1);
const SHORTCUT_TILE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::T);
const SHORTCUT_UNDO: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Z);
const SHORTCUT_REDO: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Y);

/// Bump when an existing modifier's parameters change in an incompatible way.
/// Adding brand-new modifiers does not require a bump.
const STACK_VERSION: u32 = 2;

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct StackEntry {
    id: String,
    enabled: bool,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StackFile {
    version: u32,
    #[serde(alias = "edits")]
    modifiers: Vec<StackEntry>,
}
const COMPARE_KEY: Key = Key::Tab;
const SHORTCUT_COMPARE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, COMPARE_KEY);

struct ModifierEntry {
    id: u64,
    modifier: Box<dyn Modifier>,
    enabled: bool,
}

#[derive(Default)]
pub struct KiflaApp {
    original: Option<image::RgbaImage>,
    result: Option<image::RgbaImage>,
    // Mipmapped display textures (native wgpu, registered with egui) so zoomed-
    // out previews are smooth. `*_tex` keep the GPU texture alive; `*_id` is the
    // egui handle used when painting. Rebuilt from `result`/`original` whenever
    // the corresponding `*_dirty` flag is set.
    display_tex: Option<eframe::wgpu::Texture>,
    display_id: Option<egui::TextureId>,
    display_dirty: bool,
    orig_tex: Option<eframe::wgpu::Texture>,
    orig_id: Option<egui::TextureId>,
    orig_dirty: bool,
    modifiers: Vec<ModifierEntry>,
    size: [usize; 2],
    path: Option<PathBuf>,
    error: Option<String>,
    zoom: f32,
    pan: egui::Vec2,
    recenter: bool,
    recenter_scale: f32,
    view: Option<(egui::Rect, egui::Pos2)>,
    measure_start: Option<egui::Pos2>,
    dragging: Option<usize>,
    drag_grab: f32,
    reordered: bool,
    row_heights: std::collections::HashMap<u64, f32>,
    next_id: u64,
    dirty: bool,
    unsaved: bool,
    saved_stack: Vec<StackEntry>,
    add_filter: String,
    add_selected: usize,
    add_was_open: bool,
    show_about: bool,
    tiled: bool,
    pending_load: Option<Receiver<Result<(PathBuf, image::RgbaImage), String>>>,
    pending_open: Option<Receiver<Option<PathBuf>>>,
    pending_save: Option<Receiver<Option<PathBuf>>>,
    pending_import: Option<Receiver<Option<PathBuf>>>,
    pending_export: Option<Receiver<Option<PathBuf>>>,
    gpu: Option<GpuContext>,
    // Undo/redo: snapshots of the modifier stack. `history_pos` indexes the
    // current state; undo/redo move it. A new snapshot is committed once an
    // interaction settles (pointer released), so a slider drag is one step.
    // Each entry keeps its numeric `id` so restoring reuses the same ids and
    // per-modifier UI state (collapse/expand) survives undo/redo.
    history: Vec<Vec<(u64, StackEntry)>>,
    history_pos: usize,
}

/// Sampler for the preview: crisp Nearest when magnified (zoomed in past 100%),
/// smooth trilinear (Linear + mipmaps) when minified (zoomed out) so the result
/// doesn't shimmer or alias.
fn display_sampler() -> eframe::wgpu::SamplerDescriptor<'static> {
    eframe::wgpu::SamplerDescriptor {
        label: Some("kifla.display_sampler"),
        mag_filter: eframe::wgpu::FilterMode::Nearest,
        min_filter: eframe::wgpu::FilterMode::Linear,
        mipmap_filter: eframe::wgpu::FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: 32.0,
        ..Default::default()
    }
}


impl KiflaApp {
    fn refresh_title(&self, ctx: &egui::Context) {
        let Some(name) = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
        else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title("kifla".to_owned()));
            return;
        };
        let star = if self.unsaved { "*" } else { "" };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "kifla - {star}{name} ({} × {})",
            self.size[0], self.size[1]
        )));
    }

    fn stack_entries(&self) -> Vec<StackEntry> {
        self.modifiers
            .iter()
            .map(|e| StackEntry {
                id: e.modifier.id().to_owned(),
                enabled: e.enabled,
                params: e.modifier.to_json(),
            })
            .collect()
    }

    /// Snapshot the current stack, pairing each entry with its numeric id so a
    /// later restore can reuse the same ids (preserving per-modifier UI state).
    fn stack_snapshot(&self) -> Vec<(u64, StackEntry)> {
        self.modifiers
            .iter()
            .map(|e| {
                (
                    e.id,
                    StackEntry {
                        id: e.modifier.id().to_owned(),
                        enabled: e.enabled,
                        params: e.modifier.to_json(),
                    },
                )
            })
            .collect()
    }

    /// Reset undo history to a single baseline snapshot of the current stack.
    fn reset_history(&mut self) {
        self.history = vec![self.stack_snapshot()];
        self.history_pos = 0;
    }

    /// Commit the current stack as a new history snapshot if it changed. Called
    /// once an interaction settles, so a slider drag becomes a single step.
    fn record_history(&mut self) {
        let cur = self.stack_snapshot();
        if self.history.is_empty() {
            self.history = vec![cur];
            self.history_pos = 0;
            return;
        }
        if cur != self.history[self.history_pos] {
            self.history.truncate(self.history_pos + 1);
            self.history.push(cur);
            self.history_pos = self.history.len() - 1;
        }
    }

    fn undo(&mut self, ctx: &egui::Context) {
        if self.history_pos > 0 {
            self.history_pos -= 1;
            self.restore_history(ctx);
        }
    }

    fn redo(&mut self, ctx: &egui::Context) {
        if self.history_pos + 1 < self.history.len() {
            self.history_pos += 1;
            self.restore_history(ctx);
        }
    }

    /// Rebuild the modifier stack from the snapshot at `history_pos`.
    fn restore_history(&mut self, ctx: &egui::Context) {
        let snapshot = self.history[self.history_pos].clone();
        let mut modifiers = Vec::with_capacity(snapshot.len());
        for (id, entry) in &snapshot {
            if let Some(modifier) = modifiers::modifier_from_json(&entry.id, &entry.params) {
                // Reuse the original id so egui-side per-modifier UI state
                // (collapse/expand) keyed on it survives the restore.
                self.next_id = self.next_id.max(*id);
                modifiers.push(ModifierEntry {
                    id: *id,
                    modifier,
                    enabled: entry.enabled,
                });
            }
        }
        self.modifiers = modifiers;
        // Ids are reused on restore, so cached row heights stay valid.
        self.dragging = None;
        self.dirty = true;
        self.update_unsaved(ctx);
    }

    fn update_unsaved(&mut self, ctx: &egui::Context) {
        // Disabled modifiers don't affect the image, so ignore them (and their
        // params) when deciding whether there are unsaved changes.
        let effective = |entries: &[StackEntry]| -> Vec<StackEntry> {
            entries.iter().filter(|e| e.enabled).cloned().collect()
        };
        let unsaved = effective(&self.stack_entries()) != effective(&self.saved_stack);
        if unsaved != self.unsaved {
            self.unsaved = unsaved;
            self.refresh_title(ctx);
        }
    }

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

    /// Decode an image on a background thread; the result is finalized in
    /// `update` via [`finish_load`](Self::finish_load). Keeps the UI responsive
    /// while large (e.g. 4K) files decode.
    fn start_load(&mut self, path: PathBuf) {
        if self.pending_load.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = image::open(&path)
                .map(|img| (path, img.to_rgba8()))
                .map_err(|err| format!("Failed to load image: {err}"));
            let _ = tx.send(result);
        });
        self.pending_load = Some(rx);
    }

    fn finish_load(&mut self, path: Option<PathBuf>, rgba: image::RgbaImage, ctx: &egui::Context) {
        let size = [rgba.width() as usize, rgba.height() as usize];
        self.original = Some(rgba);
        self.orig_dirty = true;
        self.size = size;
        self.path = path;
        self.error = None;
        // Start each opened image with a fresh, empty stack.
        self.modifiers.clear();
        self.row_heights.clear();
        self.dragging = None;
        self.recenter = true;
        self.recenter_scale = 0.8;
        self.unsaved = false;
        self.saved_stack = self.stack_entries();
        self.reset_history();
        self.refresh_title(ctx);
        self.rebuild();
    }

    /// Load an image straight from the clipboard (e.g. a screenshot). The result
    /// has no path, so saving routes through Save As.
    fn paste_image(&mut self, ctx: &egui::Context) {
        let image = arboard::Clipboard::new().and_then(|mut cb| cb.get_image());
        match image {
            Ok(img) => {
                let (w, h) = (img.width as u32, img.height as u32);
                match image::RgbaImage::from_raw(w, h, img.bytes.into_owned()) {
                    Some(rgba) => self.finish_load(None, rgba, ctx),
                    None => self.error = Some("Clipboard image was malformed.".to_owned()),
                }
            }
            Err(_) => self.error = Some("No image on the clipboard.".to_owned()),
        }
    }

    pub fn set_gpu(&mut self, gpu: GpuContext) {
        self.gpu = Some(gpu);
    }

    /// Rebuild and (re)register the mipmapped display textures for `result` and
    /// `original` when they change. Runs in `update` where the wgpu render state
    /// (and thus egui's texture registry) is reachable.
    fn refresh_display(&mut self, frame: &mut eframe::Frame) {
        if !self.display_dirty && !self.orig_dirty {
            return;
        }
        let Some(rs) = frame.wgpu_render_state() else {
            return;
        };

        if self.display_dirty {
            let tex = match (self.gpu.as_ref(), self.result.as_ref()) {
                (Some(gpu), Some(img)) => Some(gpu.upload_mipmapped(img)),
                _ => None,
            };
            let mut renderer = rs.renderer.write();
            if let Some(old) = self.display_id.take() {
                renderer.free_texture(&old);
            }
            self.display_tex = tex.inspect(|tex| {
                let view = tex.create_view(&Default::default());
                self.display_id = Some(renderer.register_native_texture_with_sampler_options(
                    &rs.device,
                    &view,
                    display_sampler(),
                ));
            });
            self.display_dirty = false;
        }

        if self.orig_dirty {
            let tex = match (self.gpu.as_ref(), self.original.as_ref()) {
                (Some(gpu), Some(img)) => Some(gpu.upload_mipmapped(img)),
                _ => None,
            };
            let mut renderer = rs.renderer.write();
            if let Some(old) = self.orig_id.take() {
                renderer.free_texture(&old);
            }
            self.orig_tex = tex.inspect(|tex| {
                let view = tex.create_view(&Default::default());
                self.orig_id = Some(renderer.register_native_texture_with_sampler_options(
                    &rs.device,
                    &view,
                    display_sampler(),
                ));
            });
            self.orig_dirty = false;
        }
    }

    /// Run the modifier stack on the GPU and store the result. GPU is the only
    /// path; if it's somehow unavailable the original passes through unchanged.
    fn rebuild(&mut self) {
        let Some(original) = self.original.clone() else {
            self.result = None;
            self.display_dirty = true;
            return;
        };
        let steps: Vec<crate::gpu::GpuStep> = self
            .modifiers
            .iter()
            .filter(|e| e.enabled)
            .map(|e| e.modifier.gpu_step())
            .collect();
        let result = match (&self.gpu, steps.is_empty()) {
            // No modifiers, or no GPU: the original is the result as-is.
            (_, true) | (None, _) => original,
            (Some(gpu), false) => gpu.apply(&original, &steps),
        };
        self.upload_result(result);
    }

    fn upload_result(&mut self, result: image::RgbaImage) {
        self.size = [result.width() as usize, result.height() as usize];
        self.result = Some(result);
        // The mipmapped display texture is (re)built in `update`, where the wgpu
        // render state is available.
        self.display_dirty = true;
    }

    fn save(&mut self, ctx: &egui::Context) {
        // A pasted image has no path yet; send Save straight to Save As.
        if self.result.is_some() && self.path.is_none() {
            self.save_as();
            return;
        }
        let (Some(result), Some(path)) = (&self.result, &self.path) else {
            return;
        };

        match save_image(result, path) {
            Ok(()) => {
                self.error = None;
                self.unsaved = false;
                self.saved_stack = self.stack_entries();
                self.refresh_title(ctx);
            }
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
                self.path = Some(path);
                self.error = None;
                self.unsaved = false;
                self.saved_stack = self.stack_entries();
                self.refresh_title(ctx);
            }
            Err(err) => self.error = Some(format!("Failed to save image: {err}")),
        }
    }

    fn export_stack(&mut self) {
        if self.pending_export.is_some() {
            return;
        }
        let mut dialog = rfd::FileDialog::new()
            .add_filter("kifla stack", &["kstack"])
            .set_file_name("modifiers.kstack");
        if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(dialog.save_file());
        });
        self.pending_export = Some(rx);
    }

    fn write_stack(&mut self, path: PathBuf) {
        let modifiers = self.stack_entries();
        let file = StackFile {
            version: STACK_VERSION,
            modifiers,
        };
        match serde_json::to_string_pretty(&file) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(()) => self.error = None,
                Err(err) => self.error = Some(format!("Failed to write stack: {err}")),
            },
            Err(err) => self.error = Some(format!("Failed to serialize stack: {err}")),
        }
    }

    fn import_stack(&mut self) {
        if self.pending_import.is_some() {
            return;
        }
        let dialog = rfd::FileDialog::new().add_filter("kifla stack", &["kstack"]);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(dialog.pick_file());
        });
        self.pending_import = Some(rx);
    }

    fn read_stack(&mut self, path: PathBuf, ctx: &egui::Context) {
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(err) => {
                self.error = Some(format!("Failed to read stack: {err}"));
                return;
            }
        };
        let file: StackFile = match serde_json::from_str(&text) {
            Ok(f) => f,
            Err(err) => {
                self.error = Some(format!("Not a valid stack file: {err}"));
                return;
            }
        };
        if file.version != STACK_VERSION {
            self.error = Some(format!(
                "Incompatible stack version {} (this build expects {STACK_VERSION}).",
                file.version
            ));
            return;
        }
        let mut modifiers = Vec::with_capacity(file.modifiers.len());
        for entry in file.modifiers {
            let Some(modifier) = modifiers::modifier_from_json(&entry.id, &entry.params) else {
                self.error = Some(format!("Unknown modifier \"{}\" in stack.", entry.id));
                return;
            };
            modifiers.push(ModifierEntry {
                id: self.next_id,
                modifier,
                enabled: entry.enabled,
            });
            self.next_id += 1;
        }
        self.modifiers = modifiers;
        self.error = None;
        self.update_unsaved(ctx);
        self.reset_history();
        self.rebuild();
    }

    fn close_texture(&mut self, ctx: &egui::Context) {
        self.original = None;
        self.result = None;
        self.display_dirty = true;
        self.orig_dirty = true;
        self.modifiers.clear();
        self.size = [0, 0];
        self.path = None;
        self.error = None;
        self.view = None;
        self.unsaved = false;
        self.saved_stack = Vec::new();
        self.reset_history();
        self.refresh_title(ctx);
    }

    fn add_button(
        &mut self,
        ui: &mut egui::Ui,
        loaded: bool,
        add_requested: bool,
    ) -> Option<Box<dyn Modifier>> {
        let mut result = None;
        ui.add_space(2.0);
        ui.vertical_centered(|ui| {
            let tip = format!(
                "Add a modifier  ({})",
                ui.ctx().format_shortcut(&SHORTCUT_ADD)
            );
            let add = ui
                .add(egui::Button::new("➕ Add…").min_size(egui::vec2(132.0, 0.0)))
                .on_hover_ui_at_pointer(|ui| {
                    ui.label(
                        egui::RichText::new(tip)
                            .size(11.0)
                            .color(egui::Color32::from_gray(165)),
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
            let count = add_modifier_matches(&self.add_filter);
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
                        add_modifier_list(ui, loaded, filter, selected, activate, scroll_to_sel)
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
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.refresh_display(frame);
        let loaded = self.result.is_some();
        let mut open_requested = false;
        let mut save_requested = false;
        let mut save_as_requested = false;
        let mut close_requested = false;
        let mut quit_requested = false;
        let mut import_requested = false;
        let mut export_requested = false;
        let mut add_modifier: Option<Box<dyn Modifier>> = None;
        let mut modifiers_dirty = false;

        let mut recenter_requested = false;
        let mut add_requested = false;
        let mut undo_requested = false;
        let mut redo_requested = false;
        let mut paste_requested = false;
        let typing = ctx.memory(|m| m.focus().is_some());
        ctx.input_mut(|i| {
            open_requested |= i.consume_shortcut(&SHORTCUT_OPEN);
            quit_requested |= i.consume_shortcut(&SHORTCUT_QUIT);
            if i.consume_shortcut(&SHORTCUT_ABOUT) {
                self.show_about = true;
            }
            // egui-winit turns the Ctrl+V press into a text-paste event (or drops
            // it for an image clipboard), so trigger on the V release with Ctrl
            // held. Skip while a text field is focused, where Ctrl+V pastes text.
            if !typing {
                paste_requested |= i.events.iter().any(|e| {
                    matches!(
                        e,
                        egui::Event::Key { key: Key::V, pressed: false, modifiers, .. }
                            if modifiers.command
                    )
                });
            }
            if loaded {
                // Consume Save As (Ctrl+Shift+S) before Save (Ctrl+S): egui's
                // Ctrl+S shortcut also fires while Shift is held, so the more
                // specific one must claim the event first.
                save_as_requested |= i.consume_shortcut(&SHORTCUT_SAVE_AS);
                save_requested |= i.consume_shortcut(&SHORTCUT_SAVE);
                close_requested |= i.consume_shortcut(&SHORTCUT_CLOSE);
                recenter_requested |= i.consume_shortcut(&SHORTCUT_RECENTER);
                if i.consume_shortcut(&SHORTCUT_TILE) {
                    self.tiled = !self.tiled;
                }
                if !typing {
                    add_requested |= i.consume_shortcut(&SHORTCUT_ADD);
                    undo_requested |= i.consume_shortcut(&SHORTCUT_UNDO);
                    redo_requested |= i.consume_shortcut(&SHORTCUT_REDO);
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
            self.start_load(path);
        }

        match self.pending_open.as_ref().map(Receiver::try_recv) {
            Some(Ok(result)) => {
                self.pending_open = None;
                if let Some(path) = result {
                    self.start_load(path);
                }
            }
            Some(Err(TryRecvError::Empty)) => ctx.request_repaint(),
            Some(Err(TryRecvError::Disconnected)) => self.pending_open = None,
            None => {}
        }
        match self.pending_load.as_ref().map(Receiver::try_recv) {
            Some(Ok(result)) => {
                self.pending_load = None;
                match result {
                    Ok((path, rgba)) => self.finish_load(Some(path), rgba, ctx),
                    Err(err) => self.error = Some(err),
                }
            }
            Some(Err(TryRecvError::Empty)) => ctx.request_repaint(),
            Some(Err(TryRecvError::Disconnected)) => self.pending_load = None,
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
        match self.pending_import.as_ref().map(Receiver::try_recv) {
            Some(Ok(result)) => {
                self.pending_import = None;
                if let Some(path) = result {
                    self.read_stack(path, ctx);
                }
            }
            Some(Err(TryRecvError::Empty)) => ctx.request_repaint(),
            Some(Err(TryRecvError::Disconnected)) => self.pending_import = None,
            None => {}
        }
        match self.pending_export.as_ref().map(Receiver::try_recv) {
            Some(Ok(result)) => {
                self.pending_export = None;
                if let Some(path) = result {
                    self.write_stack(path);
                }
            }
            Some(Err(TryRecvError::Empty)) => ctx.request_repaint(),
            Some(Err(TryRecvError::Disconnected)) => self.pending_export = None,
            None => {}
        }

        let loaded = self.result.is_some();

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    ui.style_mut().wrap = Some(false);
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
                            egui::Button::new("💾 Save")
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
                            egui::Button::new("✖ Close")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_CLOSE)),
                        )
                        .clicked()
                    {
                        close_requested = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(loaded, egui::Button::new("📥 Import Modifier Stack…"))
                        .clicked()
                    {
                        import_requested = true;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(loaded, egui::Button::new("📤 Export Modifier Stack…"))
                        .clicked()
                    {
                        export_requested = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add(
                            egui::Button::new("🚪 Quit")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_QUIT)),
                        )
                        .clicked()
                    {
                        quit_requested = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    ui.style_mut().wrap = Some(false);
                    let can_undo = self.history_pos > 0;
                    let can_redo = self.history_pos + 1 < self.history.len();
                    if ui
                        .add_enabled(
                            loaded && can_undo,
                            egui::Button::new("↩ Undo")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_UNDO)),
                        )
                        .clicked()
                    {
                        undo_requested = true;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            loaded && can_redo,
                            egui::Button::new("↪ Redo")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_REDO)),
                        )
                        .clicked()
                    {
                        redo_requested = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.style_mut().wrap = Some(false);
                    if ui
                        .add_enabled(
                            loaded,
                            egui::Button::new("🔍 Recenter")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_RECENTER)),
                        )
                        .clicked()
                    {
                        recenter_requested = true;
                        ui.close_menu();
                    }
                    let original = ui.add_enabled(
                        loaded,
                        egui::Button::new("👁 Show Original")
                            .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_COMPARE)),
                    );
                    if original.is_pointer_button_down_on() {
                        compare_held = true;
                    }
                    if ui
                        .add_enabled(
                            loaded,
                            egui::Button::new(if self.tiled {
                                "🧩 Tile Preview ✓"
                            } else {
                                "🧩 Tile Preview"
                            })
                            .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_TILE)),
                        )
                        .clicked()
                    {
                        self.tiled = !self.tiled;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Transform", |ui| {
                    if let Some(op) = modifier_menu(ui, modifiers::TRANSFORM_GROUPS, loaded) {
                        add_modifier = Some(op);
                    }
                });
                ui.menu_button("Image", |ui| {
                    if let Some(op) = modifier_menu(ui, modifiers::IMAGE_GROUPS, loaded) {
                        add_modifier = Some(op);
                    }
                });
                ui.menu_button("Help", |ui| {
                    ui.style_mut().wrap = Some(false);
                    if ui
                        .add(
                            egui::Button::new("ℹ About")
                                .shortcut_text(ui.ctx().format_shortcut(&SHORTCUT_ABOUT)),
                        )
                        .clicked()
                    {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });
            });
        });

        if self.show_about {
            let mut close = false;
            egui::Window::new("about")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_min_width(340.0);
                    ui.horizontal(|ui| {
                        ui.label("About");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("✖").clicked() {
                                close = true;
                            }
                        });
                    });
                    ui.separator();
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("kifla")
                            .size(22.0)
                            .color(egui::Color32::from_gray(125)),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("A tiny non-destructive texture editor.")
                            .color(egui::Color32::from_gray(165)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(
                            "Pile on modifiers, tweak them live, reorder or hide them whenever. \
                             Your original never gets touched until you save.",
                        )
                        .color(egui::Color32::from_gray(165)),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Made with Rust…").weak());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("…by kru.").weak());
                        });
                    });
                    ui.add_space(2.0);
                });
            if close {
                self.show_about = false;
            }
        }

        if recenter_requested {
            self.recenter = true;
            self.recenter_scale = 0.8;
        }
        let comparing = compare_held && self.orig_id.is_some();

        if loaded {
            const MODIFIERS_WIDTH: f32 = 258.0;
            egui::SidePanel::left("modifiers_panel")
                .resizable(true)
                .default_width(MODIFIERS_WIDTH)
                .width_range(80.0..=MODIFIERS_WIDTH)
                .show(ctx, |ui| {
                    ui.spacing_mut().slider_width = 150.0;
                    ui.style_mut().spacing.scroll.floating = false;
                    ui.style_mut().spacing.scroll.bar_width = 5.0;

                    ui.heading(
                        egui::RichText::new("Modifier Stack").color(egui::Color32::from_gray(120)),
                    );
                    ui.separator();

                    let mut remove_index = None;
                    let mut reset_index = None;
                    let mut set_collapse: Option<(u64, bool)> = None;
                    let mut drag_start: Option<usize> = None;
                    let mut reorder: Option<(usize, usize)> = None;
                    let mut new_grab: Option<f32> = None;
                    let mut measured: Vec<(u64, f32)> = Vec::new();
                    let dragging = self.dragging;
                    let drag_grab = self.drag_grab;
                    let result_dims = (self.size[0] as u32, self.size[1] as u32);
                    let spacing = ui.spacing().item_spacing.y;

                    let heights: Vec<f32> = self
                        .modifiers
                        .iter()
                        .map(|e| self.row_heights.get(&e.id).copied().unwrap_or(24.0))
                        .collect();
                    let mut slots = Vec::with_capacity(self.modifiers.len());
                    let mut total = 0.0;
                    for h in &heights {
                        slots.push(total);
                        total += h + spacing;
                    }

                    let pin_add = total + 36.0 > ui.available_height();
                    if pin_add {
                        let op = egui::TopBottomPanel::bottom("modifiers_add_bar")
                            .show_inside(ui, |ui| self.add_button(ui, loaded, add_requested))
                            .inner;
                        if op.is_some() {
                            add_modifier = op;
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

                            let eye_toggle = |ui: &mut egui::Ui, entry: &mut ModifierEntry| {
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

                            let name_handle = |ui: &mut egui::Ui, entry: &ModifierEntry, dim: bool| {
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
                                    entry.modifier.name(),
                                    egui::FontId::proportional(14.0),
                                    color,
                                );
                                if resp.hovered() || resp.dragged() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                }
                                resp.drag_started()
                            };

                            let order: Vec<usize> = (0..self.modifiers.len())
                                .filter(|i| Some(*i) != dragging)
                                .chain(dragging)
                                .collect();
                            for i in order {
                                let entry = &mut self.modifiers[i];
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
                                    if entry.modifier.has_settings() {
                                        let cid = egui::Id::new(("modifier_body", entry.id));
                                        egui::collapsing_header::CollapsingState::load_with_default_open(
                                            ui.ctx(),
                                            cid,
                                            false,
                                        )
                                        .show_header(ui, |ui| {
                                            ui.spacing_mut().item_spacing.x = 2.0;
                                            if eye_toggle(ui, entry) {
                                                modifiers_dirty = true;
                                                set_collapse = Some((entry.id, entry.enabled));
                                            }
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.small_button("×").clicked() {
                                                        remove_index = Some(i);
                                                    }
                                                    // Offer reset only when the modifier differs
                                                    // from a freshly-added (default) instance.
                                                    let changed = modifiers::default_modifier(
                                                        entry.modifier.id(),
                                                    )
                                                    .map(|mut d| {
                                                        d.on_added(result_dims.0, result_dims.1);
                                                        d.to_json() != entry.modifier.to_json()
                                                    })
                                                    .unwrap_or(false);
                                                    if changed
                                                        && ui
                                                            .small_button("↺")
                                                            .on_hover_text("Reset to defaults")
                                                            .clicked()
                                                    {
                                                        reset_index = Some(i);
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
                                                if entry.modifier.settings_ui(ui) {
                                                    modifiers_dirty = true;
                                                }
                                            });
                                        });
                                    } else {
                                        ui.horizontal(|ui| {
                                            ui.add_space(ui.spacing().indent);
                                            ui.spacing_mut().item_spacing.x = 2.0;
                                            if eye_toggle(ui, entry) {
                                                modifiers_dirty = true;
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
                                // Drop index = how many other rows sit above the dragged row's center.
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
                                    add_modifier = Some(op);
                                }
                            }
                        });

                    for (id, h) in measured {
                        self.row_heights.insert(id, h);
                    }
                    if let Some((id, open)) = set_collapse {
                        let cid = egui::Id::new(("modifier_body", id));
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
                        let entry = self.modifiers.remove(from);
                        self.modifiers.insert(to, entry);
                        self.dragging = Some(to);
                        self.reordered = true;
                    }
                    if !ctx.input(|i| i.pointer.primary_down()) {
                        self.dragging = None;
                        if self.reordered {
                            self.reordered = false;
                            modifiers_dirty = true;
                        }
                    }
                    if let Some(i) = remove_index {
                        self.modifiers.remove(i);
                        self.dragging = None;
                        modifiers_dirty = true;
                    }
                    if let Some(i) = reset_index {
                        let id = self.modifiers[i].modifier.id();
                        if let Some(mut fresh) = modifiers::default_modifier(id) {
                            if let Some(result) = &self.result {
                                fresh.on_added(result.width(), result.height());
                            }
                            self.modifiers[i].modifier = fresh;
                            modifiers_dirty = true;
                        }
                    }
                });
        }

        if loaded {
            egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
                ui.visuals_mut().override_text_color = Some(egui::Color32::from_gray(140));
                ui.horizontal(|ui| {
                    let mut coords = String::new();
                    let sample = if comparing {
                        &self.original
                    } else {
                        &self.result
                    };
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
                            let fit_ratio = (view.width() / image.width() as f32)
                                .min(view.height() / image.height() as f32);
                            if fit_ratio > 0.0 {
                                self.zoom / fit_ratio * 100.0
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

                let Some(result_id) = self.display_id else {
                    let area = ui.max_rect();
                    ui.painter().text(
                        egui::pos2(area.center().x, area.top() + 40.0),
                        egui::Align2::CENTER_TOP,
                        "kifla",
                        egui::FontId::proportional(112.0),
                        egui::Color32::from_gray(60),
                    );
                    if self.pending_load.is_some() {
                        ui.vertical_centered(|ui| {
                            ui.add_space((ui.available_height() / 2.0 - 20.0).max(0.0));
                            ui.add(
                                egui::Spinner::new()
                                    .size(20.0)
                                    .color(egui::Color32::from_gray(140)),
                            );
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("Loading…").weak().size(14.0));
                        });
                        return;
                    }
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
                                        .color(egui::Color32::from_gray(165)),
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
                let tex_size = egui::vec2(self.size[0] as f32, self.size[1] as f32);

                if self.recenter {
                    let fit_ratio = (rect.width() / tex_size.x).min(rect.height() / tex_size.y);
                    self.zoom = fit_ratio * self.recenter_scale;
                    self.pan = egui::Vec2::ZERO;
                }

                let ctrl = ui.input(|i| i.modifiers.ctrl);

                // Ctrl+drag measures distance instead of panning.
                if response.dragged() && !ctrl {
                    self.pan += response.drag_delta();
                    self.recenter = false;
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
                        self.recenter = false;
                    }
                }

                if ctrl && (response.hovered() || response.dragged()) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                } else if response.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                } else if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }

                let image_rect =
                    egui::Rect::from_center_size(rect.center() + self.pan, tex_size * self.zoom);
                self.view = Some((rect, image_rect.min));
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                let draw_id = match (comparing, self.orig_id) {
                    (true, Some(original)) => original,
                    _ => result_id,
                };
                let painter = ui.painter_at(rect);
                if self.tiled && image_rect.width() >= 1.0 && image_rect.height() >= 1.0 {
                    let (tw, th) = (image_rect.width(), image_rect.height());
                    let start_x = rect.left() - (rect.left() - image_rect.min.x).rem_euclid(tw);
                    let start_y = rect.top() - (rect.top() - image_rect.min.y).rem_euclid(th);
                    let mut ty = start_y;
                    let mut rows = 0;
                    while ty < rect.bottom() && rows < 256 {
                        let mut tx = start_x;
                        let mut cols = 0;
                        while tx < rect.right() && cols < 256 {
                            let r =
                                egui::Rect::from_min_size(egui::pos2(tx, ty), image_rect.size());
                            painter.image(draw_id, r, uv, egui::Color32::WHITE);
                            tx += tw;
                            cols += 1;
                        }
                        ty += th;
                        rows += 1;
                    }
                } else {
                    painter.image(draw_id, image_rect, uv, egui::Color32::WHITE);
                }

                if let Some(cursor) = response.hover_pos() {
                    let painter = ui.painter_at(rect);
                    let guide = egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 128),
                    );
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

                // Measure tool: while Ctrl+dragging, draw a line from the press
                // point to the cursor and show the distance in image pixels.
                let zoom = self.zoom;
                let origin = image_rect.min;
                let to_image = |p: egui::Pos2| ((p - origin) / zoom).to_pos2();
                if response.drag_started() && ctrl {
                    self.measure_start = response.interact_pointer_pos().map(to_image);
                }
                if !(ctrl && response.dragged()) {
                    self.measure_start = None;
                }
                if let (Some(start_img), Some(end)) =
                    (self.measure_start, response.interact_pointer_pos())
                {
                    let painter = ui.painter_at(rect);
                    let color = egui::Color32::WHITE;
                    let start = origin + start_img.to_vec2() * zoom;
                    painter.line_segment([start, end], egui::Stroke::new(1.5, color));
                    painter.circle_filled(start, 2.5, color);
                    painter.circle_filled(end, 2.5, color);
                    let dist = (to_image(end) - start_img).length();
                    let text = format!("{dist:.1} px");
                    let pos = end + egui::vec2(12.0, 0.0);
                    let font = egui::FontId::proportional(13.0);
                    painter.text(
                        pos + egui::vec2(1.0, 1.0),
                        egui::Align2::LEFT_CENTER,
                        &text,
                        font.clone(),
                        egui::Color32::BLACK,
                    );
                    painter.text(pos, egui::Align2::LEFT_CENTER, &text, font, color);
                }

                draw_rulers(ui, full, rect, image_rect.min, self.zoom);
            });

        if open_requested {
            self.open_texture();
        }
        if paste_requested {
            self.paste_image(ctx);
        }
        if save_requested {
            self.save(ctx);
        }
        if save_as_requested {
            self.save_as();
        }
        if close_requested {
            self.close_texture(ctx);
        }
        if import_requested {
            self.import_stack();
        }
        if export_requested {
            self.export_stack();
        }
        if quit_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if let Some(mut modifier) = add_modifier {
            if let Some(result) = &self.result {
                modifier.on_added(result.width(), result.height());
            }
            let has_settings = modifier.has_settings();
            self.next_id += 1;
            let entry_id = self.next_id;
            self.modifiers.push(ModifierEntry {
                id: entry_id,
                modifier,
                enabled: true,
            });
            if has_settings {
                let id = egui::Id::new(("modifier_body", entry_id));
                let mut state =
                    egui::collapsing_header::CollapsingState::load_with_default_open(ctx, id, true);
                state.set_open(true);
                state.store(ctx);
            }
            modifiers_dirty = true;
        }
        self.dirty |= modifiers_dirty;
        if modifiers_dirty {
            self.update_unsaved(ctx);
        }

        // Undo/redo, or otherwise commit a history snapshot once the current
        // interaction settles (no mouse button held and not typing into a
        // field), so a slider drag or a typed value collapses into a single
        // undo step.
        if undo_requested {
            self.undo(ctx);
        } else if redo_requested {
            self.redo(ctx);
        } else if self.original.is_some() && !typing && !ctx.input(|i| i.pointer.any_down()) {
            self.record_history();
        }

        // Re-apply the stack on the GPU whenever something changed. It's fast
        // enough to run synchronously each frame the state is dirty.
        if self.dirty {
            self.rebuild();
            self.dirty = false;
        }
    }
}
