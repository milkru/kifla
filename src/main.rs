#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bluenoise;
mod color;
mod gpu;
mod modifier;
mod modifiers;
mod pixel;
mod widgets;

use eframe::egui;

use app::KiflaApp;

fn load_icon() -> Option<egui::IconData> {
    let image = image::load_from_memory(include_bytes!("../icon.ico"))
        .ok()?
        .to_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([800.0, 500.0])
        .with_title("kifla");
    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "kifla",
        options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "inter".to_owned(),
                egui::FontData::from_static(include_bytes!("../font/InterVariable.ttf")),
            );
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(0, "inter".to_owned());
            }
            cc.egui_ctx.set_fonts(fonts);

            let mut style = (*cc.egui_ctx.style()).clone();
            style.visuals.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.0 };
            style.visuals.slider_trailing_fill = true;
            style.visuals.panel_fill = egui::Color32::from_gray(45);
            style.visuals.window_fill = egui::Color32::from_gray(45);
            style.visuals.override_text_color = Some(egui::Color32::from_gray(235));
            style.visuals.selection.bg_fill = egui::Color32::from_gray(150);
            style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(220));
            style.interaction.selectable_labels = false;
            cc.egui_ctx.set_style(style);

            let mut app = KiflaApp::default();
            if let Some(rs) = &cc.wgpu_render_state {
                app.set_gpu(gpu::GpuContext::new(rs.device.clone(), rs.queue.clone()));
            }
            Box::new(app)
        }),
    )
}
