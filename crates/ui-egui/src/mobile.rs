//! Mobile / compact UI mode.
//!
//! When the window is narrower than [`COMPACT_BREAKPOINT`] or the device is in portrait
//! orientation with touch input, the interface switches to a compact layout modelled after
//! AutoCAD's mobile app: full-screen drawing, a thin top bar with the document name, and a
//! bottom toolbar with large touch targets (min 44 px) that open bottom sheets for each
//! tool category (Draw, Modify, Layers, Properties, Measure, Undo/Redo, More).
//!
//! Desktop behaviour is untouched; the compact mode is additive.

use egui::{Rect, Sense, pos2, vec2};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::CadApp;
use crate::icons::{self, Icon};
use crate::palettes::Tool;
use crate::theme::Tokens;

/// Window width below which compact mode kicks in (px).
pub const COMPACT_BREAKPOINT: f32 = 760.0;

/// Minimum touch target size (Apple HIG / Material, in points).
pub const TOUCH_MIN: f32 = 44.0;

/// Bottom toolbar height.
pub const BOTTOM_BAR_H: f32 = 56.0;

/// Top bar height in compact mode.
pub const TOP_BAR_H: f32 = 40.0;

/// Maximum bottom sheet height as a fraction of available space.
pub const SHEET_MAX_FRAC: f32 = 0.55;

/// Which bottom sheet (if any) is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sheet {
    #[default]
    None,
    Draw,
    Modify,
    Layers,
    Properties,
    Measure,
    UndoRedo,
    More,
    CommandLine,
}

/// Whether the UI should be in compact mode right now.
pub fn is_compact(ctx: &egui::Context) -> bool {
    let screen = ctx.input(|i| i.viewport_rect());
    let w = screen.width();
    let h = screen.height();
    let is_portrait = h > w;
    let is_touch = ctx.input(|i| i.any_touches() || i.raw.events.iter().any(|e| matches!(e, egui::Event::Touch { .. })));

    w < COMPACT_BREAKPOINT || (is_portrait && (is_touch || w < 900.0)) || (h < 500.0 && (is_touch || w < 900.0))
}

/// Top bar: document name (centred), hamburger/menu on the left, quick actions on the right.
pub fn top_bar(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::top("cc_mobile_top").exact_size(TOP_BAR_H).frame(egui::Frame::NONE.fill(t.chrome)).show(ui, |ui| {
        let r = ui.max_rect();
        let title = match app.session.state() {
            Ok(st) if !app.ui.start_tab => st.title.clone(),
            _ => "CADCraft".to_string(),
        };
        let galley = ui.painter().layout_no_wrap(title, egui::FontId::proportional(14.0), t.text);
        let x = (r.center().x - galley.size().x / 2.0).max(r.left() + 48.0);
        ui.painter().galley(pos2(x, r.center().y - galley.size().y / 2.0), galley, t.text);

        // Hamburger / menu button on the left.
        let btn = Rect::from_min_size(pos2(r.left() + 4.0, r.top() + 2.0), vec2(36.0, 36.0));
        let resp = ui.interact(btn, ui.id().with("mobile_menu"), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(btn, 4.0, t.control_hover.gamma_multiply(0.5));
        }
        icons::paint(ui.painter(), btn.shrink(8.0), Icon::Menu, false);
        icons::describe_control(ui, &resp, crate::tl!("Menu"), None, false);
        if resp.clicked() {
            app.ui.in_window_menu = !app.ui.in_window_menu;
        }

        // Quick command-line button and undo/redo on the right.
        let mut rx = r.right() - 6.0;
        for (icon, cmd, tip) in [(Icon::Redo, "redo", crate::tl!("Redo")), (Icon::Undo, "u", crate::tl!("Undo"))] {
            rx -= 36.0;
            let br = Rect::from_min_size(pos2(rx, r.top() + 2.0), vec2(34.0, 36.0));
            let resp = ui.interact(br, ui.id().with(("mobile_tb", cmd)), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(br, 4.0, t.control_hover.gamma_multiply(0.5));
            }
            icons::paint(ui.painter(), br.shrink(7.0), icon, false);
            icons::describe_control(ui, &resp, tip, None, false);
            if resp.clicked() {
                app.start(cmd);
            }
        }

        // Command line launcher icon.
        rx -= 36.0;
        let cmd_br = Rect::from_min_size(pos2(rx, r.top() + 2.0), vec2(34.0, 36.0));
        let cmd_resp = ui.interact(cmd_br, ui.id().with("mobile_tb_cmd"), Sense::click());
        if cmd_resp.hovered() {
            ui.painter().rect_filled(cmd_br, 4.0, t.control_hover.gamma_multiply(0.5));
        }
        ui.painter().text(cmd_br.center(), egui::Align2::CENTER_CENTER, ">_", crate::theme::mono(), t.text);
        icons::describe_control(ui, &cmd_resp, crate::tl!("Command Line"), None, false);
        if cmd_resp.clicked() {
            app.ui.mobile_sheet = if app.ui.mobile_sheet == Sheet::CommandLine { Sheet::None } else { Sheet::CommandLine };
        }
    });
}

/// The 7 bottom toolbar items as specified:
/// Rysuj, Zmień, Warstwy, Właściwości, Pomiar, Cofnij/Ponów, Więcej.
const BOTTOM_ITEMS: &[(Icon, &str, Sheet)] = &[
    (Icon::Line, "Draw", Sheet::Draw),
    (Icon::Move, "Modify", Sheet::Modify),
    (Icon::Layers, "Layers", Sheet::Layers),
    (Icon::Properties, "Properties", Sheet::Properties),
    (Icon::Measure, "Measure", Sheet::Measure),
    (Icon::Undo, "Undo / Redo", Sheet::UndoRedo),
    (Icon::Help, "More", Sheet::More),
];

/// Bottom toolbar with large touch-friendly icons (min 44 px).
pub fn bottom_toolbar(ui: &mut egui::Ui, open_sheet: &mut Sheet) {
    let t = Tokens::get();
    egui::Panel::bottom("cc_mobile_bottom").exact_size(BOTTOM_BAR_H).frame(egui::Frame::NONE.fill(t.chrome)).show(ui, |ui| {
        let r = ui.max_rect();
        let n = BOTTOM_ITEMS.len() as f32;
        let item_w = (r.width() / n).max(TOUCH_MIN);
        let total = item_w * n;
        let mut x = r.left() + (r.width() - total).max(0.0) / 2.0;

        for (icon, label, sheet) in BOTTOM_ITEMS {
            let active = *open_sheet == *sheet;
            let br = Rect::from_min_size(pos2(x, r.top()), vec2(item_w, r.height()));
            let resp = ui.interact(br, ui.id().with(("mob_bt", *label)), Sense::click());

            if active {
                ui.painter().rect_filled(br, 0.0, t.toggle_on.gamma_multiply(0.2));
            } else if resp.hovered() {
                ui.painter().rect_filled(br, 0.0, t.control_hover.gamma_multiply(0.4));
            }

            let icon_size = 24.0;
            let icon_rect = Rect::from_center_size(pos2(br.center().x, br.top() + 6.0 + icon_size / 2.0), vec2(icon_size, icon_size));
            icons::paint(ui.painter(), icon_rect, *icon, false);

            let translated = crate::i18n::t(label);
            ui.painter().text(
                pos2(br.center().x, br.bottom() - 6.0),
                egui::Align2::CENTER_BOTTOM,
                translated,
                egui::FontId::proportional(10.0),
                if active { t.toggle_on } else { t.text_dim },
            );

            icons::describe_control(ui, &resp, translated, Some(active), false);

            if resp.clicked() {
                *open_sheet = if active { Sheet::None } else { *sheet };
            }
            x += item_w;
        }
    });
}

/// Draw the bottom sheet that slides up from the bottom toolbar.
pub fn bottom_sheet(app: &mut CadApp, ui: &mut egui::Ui, sheet: Sheet) {
    if sheet == Sheet::None {
        return;
    }
    let t = Tokens::get();
    let avail = ui.available_rect_before_wrap();
    let max_h = (avail.height() * SHEET_MAX_FRAC).min(420.0);
    let sheet_h = match sheet {
        Sheet::CommandLine => 200.0_f32.min(max_h),
        Sheet::UndoRedo => 220.0_f32.min(max_h),
        Sheet::Measure => 200.0_f32.min(max_h),
        Sheet::More => 260.0_f32.min(max_h),
        _ => 320.0_f32.min(max_h),
    };

    egui::Panel::bottom("cc_mobile_sheet").exact_size(sheet_h).frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::same(8))).show(
        ui,
        |ui| {
            // Drag handle at top.
            let r = ui.max_rect();
            let handle = Rect::from_min_size(pos2(r.center().x - 20.0, r.top() + 3.0), vec2(40.0, 4.0));
            ui.painter().rect_filled(handle, 2.0, t.text_faint);

            // Close button on top-right of sheet.
            let close_r = Rect::from_min_size(pos2(r.right() - 28.0, r.top() + 2.0), vec2(24.0, 24.0));
            let close_resp = ui.interact(close_r, ui.id().with("sheet_close"), Sense::click());
            if close_resp.hovered() {
                ui.painter().rect_filled(close_r, 4.0, t.control_hover.gamma_multiply(0.5));
            }
            icons::paint(ui.painter(), close_r.shrink(6.0), Icon::Close, false);
            icons::describe_control(ui, &close_resp, crate::tl!("Close"), None, false);
            if close_resp.clicked() {
                app.ui.mobile_sheet = Sheet::None;
                return;
            }

            ui.add_space(14.0);

            match sheet {
                Sheet::Draw => draw_sheet(app, ui),
                Sheet::Modify => modify_sheet(app, ui),
                Sheet::Layers => layers_sheet(app, ui),
                Sheet::Properties => properties_sheet(app, ui),
                Sheet::Measure => measure_sheet(app, ui),
                Sheet::UndoRedo => undo_redo_sheet(app, ui),
                Sheet::More => more_sheet(app, ui),
                Sheet::CommandLine => command_line_sheet(app, ui),
                Sheet::None => {}
            }
        },
    );
}

/// Draw tools sheet - reuses existing DRAFTING tool definitions.
fn draw_sheet(app: &mut CadApp, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new(crate::tl!("Draw")).strong().size(14.0));
    ui.add_space(4.0);
    let mut clicked: Option<&'static str> = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let draw_tools: &[Tool] = &[
            (Icon::Line, "line", "Line"),
            (Icon::Polyline, "pline", "Polyline"),
            (Icon::Circle, "circle", "Circle"),
            (Icon::Arc, "arc", "Arc"),
            (Icon::Rectangle, "rectang", "Rectangle"),
            (Icon::Polygon, "polygon", "Polygon"),
            (Icon::Ellipse, "ellipse", "Ellipse"),
            (Icon::Spline, "spline", "Spline"),
            (Icon::Point, "point.multiple", "Multiple Point"),
            (Icon::Donut, "donut", "Donut"),
            (Icon::Hatch, "hatch", "Hatch"),
            (Icon::Text, "text", "Single Line Text"),
            (Icon::MText, "mtext", "Multiline Text"),
            (Icon::RevCloud, "revcloud", "Revision Cloud"),
            (Icon::Region, "region", "Region"),
            (Icon::Table, "table", "Table"),
        ];
        tool_grid(ui, draw_tools, &mut clicked);
    });
    if let Some(c) = clicked {
        app.cmd.buffer.clear();
        app.start(c);
        app.ui.mobile_sheet = Sheet::None;
    }
}

/// Modify tools sheet.
fn modify_sheet(app: &mut CadApp, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new(crate::tl!("Modify")).strong().size(14.0));
    ui.add_space(4.0);
    let mut clicked: Option<&'static str> = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let modify_tools: &[Tool] = &[
            (Icon::Move, "move", "Move"),
            (Icon::Copy, "copy", "Copy"),
            (Icon::Rotate, "rotate", "Rotate"),
            (Icon::Scale, "scale", "Scale"),
            (Icon::Mirror, "mirror", "Mirror"),
            (Icon::Offset, "offset", "Offset"),
            (Icon::Trim, "trim", "Trim"),
            (Icon::Extend, "extend", "Extend"),
            (Icon::Fillet, "fillet", "Fillet"),
            (Icon::Chamfer, "chamfer", "Chamfer"),
            (Icon::Explode, "explode", "Explode"),
            (Icon::Erase, "erase", "Erase"),
            (Icon::Stretch, "stretch", "Stretch"),
            (Icon::ArrayRect, "arrayrect", "Rectangular Array"),
            (Icon::ArrayPolar, "arraypolar", "Polar Array"),
            (Icon::Join, "join", "Join"),
            (Icon::Break, "break", "Break"),
            (Icon::MatchProp, "matchprop", "Match Properties"),
        ];
        tool_grid(ui, modify_tools, &mut clicked);
    });
    if let Some(c) = clicked {
        app.cmd.buffer.clear();
        app.start(c);
        app.ui.mobile_sheet = Sheet::None;
    }
}

/// Layers sheet - reuses existing layers_section.
fn layers_sheet(app: &mut CadApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        crate::palettes::layers_section(app, ui);
    });
}

/// Properties sheet - reuses existing properties_section.
fn properties_sheet(app: &mut CadApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        crate::palettes::properties_section(app, ui);
    });
}

/// Measure tools sheet.
fn measure_sheet(app: &mut CadApp, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new(crate::tl!("Measure")).strong().size(14.0));
    ui.add_space(4.0);
    let mut clicked: Option<&'static str> = None;
    let tools: &[Tool] = &[(Icon::Measure, "dist", "Measure Distance"), (Icon::Area, "area", "Area"), (Icon::List, "list", "List")];
    tool_grid(ui, tools, &mut clicked);
    if let Some(c) = clicked {
        app.cmd.buffer.clear();
        app.start(c);
        app.ui.mobile_sheet = Sheet::None;
    }
}

/// Undo / Redo sheet with buttons and recent command history.
fn undo_redo_sheet(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    ui.label(egui::RichText::new(crate::tl!("Undo / Redo")).strong().size(14.0));
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(12.0, 0.0);
        let btn_w = (ui.available_width() - 12.0) / 2.0;

        let (u_rect, u_resp) = ui.allocate_exact_size(vec2(btn_w, 44.0), Sense::click());
        if u_resp.hovered() {
            ui.painter().rect_filled(u_rect, 6.0, t.control_hover);
        } else {
            ui.painter().rect_filled(u_rect, 6.0, t.control);
        }
        icons::paint(ui.painter(), Rect::from_center_size(pos2(u_rect.left() + 24.0, u_rect.center().y), vec2(20.0, 20.0)), Icon::Undo, false);
        ui.painter().text(pos2(u_rect.left() + 48.0, u_rect.center().y), egui::Align2::LEFT_CENTER, crate::tl!("Undo"), crate::theme::body(), t.text);
        icons::describe_control(ui, &u_resp, crate::tl!("Undo"), None, false);
        if u_resp.clicked() {
            app.start("u");
        }

        let (r_rect, r_resp) = ui.allocate_exact_size(vec2(btn_w, 44.0), Sense::click());
        if r_resp.hovered() {
            ui.painter().rect_filled(r_rect, 6.0, t.control_hover);
        } else {
            ui.painter().rect_filled(r_rect, 6.0, t.control);
        }
        icons::paint(ui.painter(), Rect::from_center_size(pos2(r_rect.left() + 24.0, r_rect.center().y), vec2(20.0, 20.0)), Icon::Redo, false);
        ui.painter().text(pos2(r_rect.left() + 48.0, r_rect.center().y), egui::Align2::LEFT_CENTER, crate::tl!("Redo"), crate::theme::body(), t.text);
        icons::describe_control(ui, &r_resp, crate::tl!("Redo"), None, false);
        if r_resp.clicked() {
            app.start("redo");
        }
    });

    ui.add_space(8.0);
    let lines: Vec<String> = app.session.log.iter().rev().take(4).rev().cloned().collect();
    if !lines.is_empty() {
        ui.label(egui::RichText::new(crate::tl!("Recent Commands")).size(12.0).color(t.text_dim));
        for l in &lines {
            ui.label(egui::RichText::new(l).size(11.0).color(t.text_faint));
        }
    }
}

/// More sheet: quick navigation, file ops, command line launcher.
fn more_sheet(app: &mut CadApp, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new(crate::tl!("More")).strong().size(14.0));
    ui.add_space(4.0);
    let mut clicked: Option<&'static str> = None;
    let mut open_cmd = false;

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        // Command line toggle row.
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
        let t = Tokens::get();
        if resp.hovered() {
            ui.painter().rect_filled(rect, 4.0, t.control_hover.gamma_multiply(0.5));
        } else {
            ui.painter().rect_filled(rect, 4.0, t.control.gamma_multiply(0.5));
        }
        ui.painter().text(pos2(rect.left() + 14.0, rect.center().y), egui::Align2::LEFT_CENTER, ">_  ", crate::theme::mono(), t.text);
        ui.painter().text(
            pos2(rect.left() + 42.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            crate::tl!("Command Line"),
            crate::theme::body(),
            t.text,
        );
        icons::describe_control(ui, &resp, crate::tl!("Command Line"), None, false);
        if resp.clicked() {
            open_cmd = true;
        }

        ui.add_space(6.0);

        let tools: &[Tool] = &[
            (Icon::ZoomExtents, "zoom.extents", "Zoom Extents"),
            (Icon::ZoomWindow, "zoom.window", "Zoom Window"),
            (Icon::Pan, "pan", "Pan"),
            (Icon::New, "new", "New Drawing"),
            (Icon::Open, "open", "Open"),
            (Icon::Save, "qsave", "Save"),
            (Icon::SaveAs, "saveas", "Save As"),
            (Icon::Plot, "plot", "Print"),
            (Icon::Help, "ui.dialog.about", "Help"),
        ];
        tool_grid(ui, tools, &mut clicked);
    });

    if open_cmd {
        app.ui.mobile_sheet = Sheet::CommandLine;
    } else if let Some(c) = clicked {
        app.cmd.buffer.clear();
        app.start(c);
        app.ui.mobile_sheet = Sheet::None;
    }
}

/// Command line sheet (minimal touch-friendly input with prompt display).
fn command_line_sheet(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    ui.label(egui::RichText::new(crate::tl!("Command Line")).strong().size(14.0));
    ui.add_space(4.0);

    // Show the last few history lines.
    let lines: Vec<String> = app.session.log.iter().rev().take(4).rev().cloned().collect();
    for l in &lines {
        ui.label(egui::RichText::new(l).size(11.0).color(t.text_dim));
    }

    // Show the current prompt.
    if let Some(pr) = app.session.current_prompt() {
        let msg = if pr.raw { pr.display() } else { crate::i18n::t(&pr.message).to_string() };
        ui.label(egui::RichText::new(msg).size(12.0).color(t.text));
    }

    ui.add_space(4.0);

    // Text input.
    let resp = ui.add_sized(
        [ui.available_width(), 32.0],
        egui::TextEdit::singleline(&mut app.cmd.buffer)
            .font(crate::theme::body())
            .hint_text(crate::tl!("Type a command"))
            .id(ui.id().with("mobile_cmd_input")),
    );
    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        let text = app.cmd.buffer.clone();
        app.cmd.buffer.clear();
        app.cmdline(&text);
        resp.request_focus();
    }
}

/// A touch-friendly grid of tool buttons (large icons >= 44 px, labels below).
fn tool_grid(ui: &mut egui::Ui, tools: &[Tool], clicked: &mut Option<&'static str>) {
    let t = Tokens::get();
    let avail_w = ui.available_width();
    let cols = ((avail_w / 72.0) as usize).clamp(3, 6);
    let cell_w = avail_w / cols as f32;
    let cell_h = 64.0;

    egui::Grid::new(ui.id().with("mobile_tool_grid")).num_columns(cols).spacing(vec2(2.0, 4.0)).show(ui, |ui| {
        for (i, (icon, cmd, label)) in tools.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(vec2(cell_w - 4.0, cell_h), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, 6.0, t.control_hover.gamma_multiply(0.4));
            }
            let icon_rect = Rect::from_center_size(pos2(rect.center().x, rect.top() + 24.0), vec2(28.0, 28.0));
            icons::paint(ui.painter(), icon_rect, *icon, false);
            let translated = crate::i18n::t(label);
            ui.painter().text(
                pos2(rect.center().x, rect.bottom() - 4.0),
                egui::Align2::CENTER_BOTTOM,
                translated,
                egui::FontId::proportional(10.5),
                t.text,
            );
            icons::describe_control(ui, &resp, translated, None, false);
            if resp.clicked() {
                *clicked = Some(cmd);
            }
            if (i + 1) % cols == 0 {
                ui.end_row();
            }
        }
    });
}

/// File and Model/Layout tabs in compact mode: horizontally scrollable, compact.
pub fn compact_file_tabs(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    let has_docs = !app.session.docs.is_empty();
    if !has_docs {
        return;
    }
    egui::Panel::top("cc_mobile_tabs").exact_size(26.0).frame(egui::Frame::NONE.fill(t.chrome_dark)).show(ui, |ui| {
        egui::ScrollArea::horizontal().id_salt("mobile_tabs").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                let mut switch_to = None;
                let titles: Vec<(usize, String, bool, bool)> =
                    app.session.docs.iter().enumerate().map(|(i, d)| (i, d.title.clone(), d.is_dirty(), i == app.session.active)).collect();

                // Model / Layout tabs.
                if let Ok(st) = app.session.state() {
                    let space = match &st.space {
                        cadcraft_doc::Space::Model => "Model".to_string(),
                        cadcraft_doc::Space::Paper(n) => n.clone(),
                    };
                    let mut tabs = vec!["Model".to_string()];
                    let mut ls: Vec<_> = st.doc.layouts.iter().collect();
                    ls.sort_by_key(|l| l.tab_order);
                    tabs.extend(ls.iter().map(|l| l.name.clone()));

                    for name in &tabs {
                        let active = *name == space;
                        let label = if name == "Model" { crate::tl!("Model").to_string() } else { name.clone() };
                        let btn = ui.selectable_label(active, &label);
                        if btn.clicked() && !active {
                            let _ = app.run("layout.set", json!({ "name": name }));
                        }
                    }
                }

                ui.separator();

                for (i, title, dirty, active) in titles {
                    let mark = if dirty { "*" } else { "" };
                    let label = format!("{title}{mark}");
                    let btn = ui.selectable_label(active && !app.ui.start_tab, &label);
                    if btn.clicked() {
                        switch_to = Some(i);
                    }
                }

                if let Some(i) = switch_to {
                    let _ = app.run("document.switch", json!({ "index": i }));
                    app.ui.start_tab = false;
                }
            });
        });
    });
}

/// The full compact-mode draw method: replaces the desktop `draw` when compact mode is active.
pub fn draw_compact(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    crate::control::begin_frame(ui.ctx());
    top_bar(app, ui);

    // Menu bar (toggled by hamburger).
    if app.ui.in_window_menu {
        crate::menus::menu_bar(app, ui);
    }

    // Compact file/layout tabs.
    if app.ui.show_file_tabs {
        compact_file_tabs(app, ui);
    }

    // Bottom toolbar + optional sheet.
    let mut sheet = app.ui.mobile_sheet;
    bottom_toolbar(ui, &mut sheet);
    bottom_sheet(app, ui, sheet);
    app.ui.mobile_sheet = sheet;

    // Central drawing area: full screen, no side panels.
    let has_doc = !app.session.docs.is_empty() && !app.ui.start_tab;
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |ui| {
        if has_doc {
            crate::docking::show(app, ui);
        } else {
            crate::chrome::start_page(app, ui);
        }
    });

    crate::dialogs::show(app, ui.ctx());
    crate::palettes::patterns::prompt_picker(app, ui.ctx());
    crate::closing::prompt(app, ui.ctx());
    crate::autosave::show(app, ui.ctx());
    crate::autosave::gpu_notice(app, ui.ctx());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_breakpoint_is_reasonable() {
        const { assert!(COMPACT_BREAKPOINT > 0.0) };
        const { assert!(COMPACT_BREAKPOINT < 1200.0) };
    }

    #[test]
    fn sheet_default_is_none() {
        assert_eq!(Sheet::default(), Sheet::None);
    }

    #[test]
    fn bottom_items_all_have_labels() {
        for (_, label, _) in BOTTOM_ITEMS {
            assert!(!label.is_empty());
        }
    }

    #[test]
    fn touch_min_meets_hig() {
        const { assert!(TOUCH_MIN >= 44.0) };
    }
}
