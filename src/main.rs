// Kifla — Texture Lab
//
// A lightweight desktop application for processing textures. See README.md for
// the overall design: load a texture, apply full-image operations as a
// non-destructive history stack, preview the result, and save it.

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("Kifla — Texture Lab"),
        ..Default::default()
    };

    eframe::run_native(
        "Kifla",
        options,
        Box::new(|_cc| Box::<KiflaApp>::default()),
    )
}

#[derive(Default)]
struct KiflaApp {}

impl eframe::App for KiflaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top menu bar: file actions and available tools.
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open…").clicked() {
                        ui.close_menu();
                    }
                    if ui.button("Save…").clicked() {
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Tools", |_ui| {});
            });
        });

        // History panel: original texture and applied operations.
        egui::SidePanel::left("history_panel")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("History");
                ui.separator();
                ui.label("No texture loaded.");
            });

        // Tool settings panel: configures the currently selected operation.
        egui::SidePanel::right("tool_settings_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Tool Settings");
                ui.separator();
                ui.label("No tool selected.");
            });

        // Central preview area.
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label("Open a texture to get started (File → Open…).");
            });
        });
    }
}
