use eframe::egui;

/// A numeric box that commits live while dragging or typing, and can be
/// fine-tuned by ±1 with Ctrl+scroll while hovered. Returns true when the
/// value should be committed.
pub fn drag_value<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut Num,
    range: std::ops::RangeInclusive<Num>,
) -> bool {
    let response = ui.add(egui::DragValue::new(value).clamp_range(range.clone()));
    let mut commit = response.changed();
    commit |= fine_tune(ui, &response, value, range);
    commit
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
/// the value by `step` per notch and committing immediately. Returns true when
/// the value should commit.
pub fn fine_tune_step<Num: egui::emath::Numeric>(
    ui: &egui::Ui,
    response: &egui::Response,
    value: &mut Num,
    range: std::ops::RangeInclusive<Num>,
    step: f64,
) -> bool {
    let mut commit = false;

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
                commit = true;
            }
        }
    }

    commit || response.drag_released() || response.lost_focus()
}

/// Cycles a dropdown's value with Ctrl+scroll while its (closed) header is
/// hovered - scroll up for the previous option, down for the next, wrapping
/// around. `response` is the `ComboBox::show_ui` header response. Returns true
/// when the value changed.
pub fn combo_scroll<T: PartialEq + Copy>(
    ui: &egui::Ui,
    response: &egui::Response,
    value: &mut T,
    all: &[T],
) -> bool {
    if !response.hovered() || all.is_empty() {
        return false;
    }
    let dy = ui.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::MouseWheel {
                delta, modifiers, ..
            } if modifiers.ctrl => Some(delta.y),
            _ => None,
        })
    });
    let Some(dy) = dy else { return false };
    if dy == 0.0 {
        return false;
    }
    let Some(index) = all.iter().position(|v| v == value) else {
        return false;
    };
    let step = if dy > 0.0 { -1 } else { 1 };
    let next = (index as i32 + step).rem_euclid(all.len() as i32) as usize;
    if next != index {
        *value = all[next];
        true
    } else {
        false
    }
}

/// A clean Ctrl+scroll step (1, 2, or 5 times a power of ten) sized to the
/// range, capped at 0.1 so even wide-range sliders still nudge finely.
fn fine_step_for(range: &std::ops::RangeInclusive<f32>) -> f64 {
    let span = (range.end() - range.start()).abs() as f64;
    if span <= 0.0 {
        return 0.1;
    }
    let target = span / 250.0;
    let pow = 10f64.powf(target.log10().floor());
    let step = [1.0, 2.0, 5.0]
        .into_iter()
        .map(|m| m * pow)
        .find(|&s| s >= target)
        .unwrap_or(10.0 * pow);
    step.min(0.1)
}

/// Decimal places needed to show `step` (and finer values) without rounding it away.
fn decimals_for(step: f64) -> usize {
    if step >= 1.0 {
        0
    } else {
        (-step.log10()).ceil().max(0.0).min(6.0) as usize
    }
}

pub fn slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    let step = fine_step_for(&range);
    ui.label(format!("{}: {:.*}", label, decimals_for(step), value));

    if !ui.is_enabled() {
        return false;
    }

    let mut commit = false;
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = 7.0;
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(1.0));
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
