use eframe::egui;

/// A numeric box that commits on drag release or focus loss, and can be
/// fine-tuned by ±1 with Ctrl+scroll while hovered. Returns true when the
/// value should be committed.
pub fn drag_value<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut Num,
    range: std::ops::RangeInclusive<Num>,
) -> bool {
    let response = ui.add(egui::DragValue::new(value).clamp_range(range.clone()));
    fine_tune(ui, &response, value, range)
}

/// Adds Ctrl+scroll fine-tuning to an already-added widget response, nudging
/// the value by ±1 per notch. See [`fine_tune_step`] for a custom step.
pub fn fine_tune<Num: egui::emath::Numeric>(
    ui: &egui::Ui,
    response: &egui::Response,
    value: &mut Num,
    range: std::ops::RangeInclusive<Num>,
) -> bool {
    fine_tune_step(ui, response, value, range, 1.0)
}

/// Adds Ctrl+scroll fine-tuning to an already-added widget response, nudging
/// the value by `step` per notch. While Ctrl is held, scrolling nudges the
/// value live without committing; the commit (which triggers a re-render)
/// fires once Ctrl is released. Returns true when the value should commit.
pub fn fine_tune_step<Num: egui::emath::Numeric>(
    ui: &egui::Ui,
    response: &egui::Response,
    value: &mut Num,
    range: std::ops::RangeInclusive<Num>,
    step: f64,
) -> bool {
    let pending_id = response.id.with("ctrl_scroll_pending");

    if response.hovered() {
        let dy = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::MouseWheel {
                    delta, modifiers, ..
                } if modifiers.ctrl => Some(delta.y),
                _ => None,
            })
        });
        if let Some(dy) = dy {
            if dy != 0.0 {
                let v = (value.to_f64() + dy.signum() as f64 * step)
                    .clamp(range.start().to_f64(), range.end().to_f64());
                *value = Num::from_f64(v);
                ui.data_mut(|d| d.insert_temp(pending_id, true));
            }
        }
    }

    let pending = ui.data(|d| d.get_temp::<bool>(pending_id).unwrap_or(false));
    if pending && !ui.input(|i| i.modifiers.ctrl) {
        ui.data_mut(|d| d.insert_temp(pending_id, false));
        return true;
    }

    response.drag_released() || response.lost_focus()
}

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
        ui.spacing_mut().interact_size.y = 7.0;
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(1.0));
        let step = (range.end() - range.start()) as f64 * 0.01;
        let response = ui.add(egui::Slider::new(value, range.clone()).show_value(false));
        commit = response.changed();
        commit |= fine_tune_step(ui, &response, value, range, step);
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

    painter.line_segment(
        [
            to_screen(egui::pos2(0.0, 0.0)),
            to_screen(egui::pos2(1.0, 1.0)),
        ],
        egui::Stroke::new(1.0, grid),
    );

    let samples = 96;
    let line: Vec<egui::Pos2> = (0..=samples)
        .map(|i| {
            let x = i as f32 / samples as f32;
            to_screen(egui::pos2(x, curve_value(points, x).clamp(0.0, 1.0)))
        })
        .collect();
    painter.add(egui::Shape::line(line, egui::Stroke::new(1.5, curve_color)));

    let mut commit = false;
    let mut on_point = false;
    let mut remove = None;
    let last = points.len() - 1;
    for i in 0..points.len() {
        let center = to_screen(points[i]);
        let mut radius = 3.5;
        if enabled {
            let point_rect = egui::Rect::from_center_size(center, egui::vec2(14.0, 14.0));
            let pr = ui.interact(
                point_rect,
                response.id.with(i),
                egui::Sense::click_and_drag(),
            );
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

pub fn curve_value(points: &[egui::Pos2], x: f32) -> f32 {
    let n = points.len();
    if n == 0 {
        return x;
    }
    if x <= points[0].x {
        return points[0].y;
    }
    if x >= points[n - 1].x {
        return points[n - 1].y;
    }

    // Monotone cubic Hermite: tangents flatten at local extrema to avoid overshoot.
    let secant =
        |i: usize| (points[i + 1].y - points[i].y) / (points[i + 1].x - points[i].x).max(1e-6);
    let tangent = |i: usize| {
        if i == 0 {
            secant(0)
        } else if i == n - 1 {
            secant(n - 2)
        } else {
            let (left, right) = (secant(i - 1), secant(i));
            if left * right <= 0.0 {
                0.0
            } else {
                (left + right) * 0.5
            }
        }
    };

    let mut k = 0;
    while k + 1 < n && x > points[k + 1].x {
        k += 1;
    }

    let h = (points[k + 1].x - points[k].x).max(1e-6);
    let t = ((x - points[k].x) / h).clamp(0.0, 1.0);
    let (t2, t3) = (t * t, t * t * t);
    let m0 = tangent(k) * h;
    let m1 = tangent(k + 1) * h;

    (2.0 * t3 - 3.0 * t2 + 1.0) * points[k].y
        + (t3 - 2.0 * t2 + t) * m0
        + (-2.0 * t3 + 3.0 * t2) * points[k + 1].y
        + (t3 - t2) * m1
}
