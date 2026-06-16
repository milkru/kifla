mod app;
mod color;
mod operation;
mod operations;
mod widgets;

use eframe::egui;

use app::KiflaApp;

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
        Box::new(|cc| {
            let mut style = (*cc.egui_ctx.style()).clone();
            style.visuals.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.0 };
            style.visuals.slider_trailing_fill = true;
            style.visuals.panel_fill = egui::Color32::from_gray(45);
            style.visuals.window_fill = egui::Color32::from_gray(45);
            style.visuals.override_text_color = Some(egui::Color32::from_gray(235));
            style.visuals.selection.bg_fill = egui::Color32::from_gray(150);
            style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(220));
            cc.egui_ctx.set_style(style);
            Box::<KiflaApp>::default()
        }),
    )
}
