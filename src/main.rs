mod app;
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
            cc.egui_ctx.set_style(style);
            Box::<KiflaApp>::default()
        }),
    )
}
