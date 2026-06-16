use eframe::egui;

pub fn slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    ui.label(format!("{label}: {value:.2}"));

    if !ui.is_enabled() {
        return false;
    }

    let mut commit = false;
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = 6.0;
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(1.0));
        let response = ui.add(egui::Slider::new(value, range).show_value(false));
        commit = response.drag_released() || (response.changed() && !response.dragged());
    });
    commit
}

pub fn curve_editor(ui: &mut egui::Ui, points: &mut Vec<egui::Pos2>) -> bool {
    let side = ui.available_width().min(220.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());

    let bg = ui.visuals().extreme_bg_color;
    let grid = egui::Color32::from_gray(70);
    let curve_color = ui.visuals().text_color();
    let point_color = ui.visuals().selection.bg_fill;
    let enabled = ui.is_enabled();

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, bg);
    for i in 1..4 {
        let t = i as f32 / 4.0;
        let x = rect.left() + t * rect.width();
        let y = rect.top() + t * rect.height();
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, grid),
        );
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, grid),
        );
    }

    let to_screen = |p: egui::Pos2| {
        egui::pos2(
            rect.left() + p.x * rect.width(),
            rect.bottom() - p.y * rect.height(),
        )
    };
    let to_curve = |s: egui::Pos2| {
        egui::pos2(
            ((s.x - rect.left()) / rect.width()).clamp(0.0, 1.0),
            ((rect.bottom() - s.y) / rect.height()).clamp(0.0, 1.0),
        )
    };

    painter.add(egui::Shape::line(
        points.iter().map(|p| to_screen(*p)).collect(),
        egui::Stroke::new(1.5, curve_color),
    ));

    let mut commit = false;
    let mut on_point = false;
    let mut remove = None;
    let last = points.len() - 1;
    for i in 0..points.len() {
        let center = to_screen(points[i]);
        let mut radius = 3.5;
        if enabled {
            let point_rect = egui::Rect::from_center_size(center, egui::vec2(14.0, 14.0));
            let pr = ui.interact(point_rect, response.id.with(i), egui::Sense::click_and_drag());
            if pr.hovered() || pr.dragged() {
                on_point = true;
                radius = 5.0;
            }
            if pr.dragged() {
                let mut c = to_curve(center + pr.drag_delta());
                if i == 0 {
                    c.x = 0.0;
                } else if i == last {
                    c.x = 1.0;
                } else {
                    c.x = c.x.clamp(points[i - 1].x + 0.001, points[i + 1].x - 0.001);
                }
                points[i] = c;
            }
            if pr.drag_released() {
                commit = true;
            }
            if pr.secondary_clicked() && i != 0 && i != last {
                remove = Some(i);
            }
        }
        painter.circle_filled(center, radius, point_color);
    }

    if let Some(i) = remove {
        points.remove(i);
        commit = true;
    } else if enabled && response.clicked() && !on_point {
        if let Some(pos) = response.interact_pointer_pos() {
            let c = to_curve(pos);
            if c.x > 0.0 && c.x < 1.0 {
                points.push(c);
                points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
                commit = true;
            }
        }
    }

    commit
}
