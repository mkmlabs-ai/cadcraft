//! The Plot dialog and the other manager dialogs are drawn in the interface language. They once
//! used bare English literals, so a Polish editor kept an English "Printer/plotter" window.
//!
//! A text is a leak when it is an English catalog key whose Polish row says something else.

use cadcraft_engine::Session;
use cadcraft_ui_egui::i18n::{self, Preference};
use cadcraft_ui_egui::{CadApp, Services, menus};

fn painted_lines(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
    fn collect(shape: &egui::Shape, lines: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(s) => lines.push(s.galley.text().to_string()),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, lines)),
            _ => {}
        }
    }
    let mut lines = Vec::new();
    shapes.iter().for_each(|s| collect(&s.shape, &mut lines));
    lines
}

/// Two frames (windows fade in), returning the text painted by the last.
fn render(app: &mut CadApp) -> Vec<String> {
    let ctx = egui::Context::default();
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0))), ..Default::default() };
    let mut lines = Vec::new();
    for _ in 0..2 {
        app.logic(&ctx);
        let mut output = ctx.run_ui(input.clone(), |ui| app.ui(ui));
        lines = painted_lines(&output.shapes);
        output.textures_delta.clear();
    }
    lines
}

/// An editor started like CoreCAD's: the page asks for Polish (`?lang=pl`) while the browser
/// says English, and Options ▸ Language is still on Auto.
fn host_polish_app() -> CadApp {
    let mut app = CadApp::new(Session::new(), Services::default());
    app.system_languages = vec!["pl".into(), "en-US".into(), "en".into()];
    assert_eq!(app.ui.interface_language, Preference::Auto);
    assert_eq!(app.language(), "pl");
    app
}

fn leaks(lines: &[String]) -> Vec<&str> {
    lines.iter().map(String::as_str).filter(|l| l.chars().count() > 1 && *l != "Standard" && i18n::tr("pl", l) != *l).collect()
}

#[test]
fn main_window_is_polish_without_a_dialog() {
    let mut app = host_polish_app();
    let lines = render(&mut app);
    assert!(lines.iter().any(|l| l == "Początek") && lines.iter().any(|l| l == "Warstwy"), "the window is Polish: {lines:?}");
    assert_eq!(leaks(&lines), Vec::<&str>::new(), "English text in the Polish main window");
}

#[test]
fn plot_dialog_is_polish() {
    let mut app = host_polish_app();
    menus::activate(&mut app, "plot");
    assert_eq!(app.ui.dialog.as_deref(), Some("plot"));
    let lines = render(&mut app);
    for expected in
        ["Drukuj", "Arkusz", "Drukarka/ploter", "Rozmiar papieru", "Orientacja", "Obszar wydruku", "Dopasuj do papieru", "Tabela stylów wydruku"]
    {
        assert!(lines.iter().any(|l| l == expected), "{expected} is shown: {lines:?}");
    }
    assert!(!lines.iter().any(|l| l == "Printer/plotter" || l == "Paper size" || l == "Fit to paper"), "no English Plot window: {lines:?}");
    assert_eq!(leaks(&lines), Vec::<&str>::new());
}

#[test]
fn every_manager_dialog_is_polish() {
    for command in ["pagesetup", "linetype", "style", "tablestyle", "mleaderstyle", "ddptype", "constraintsettings", "dsettings", "units", "plot"] {
        let mut app = host_polish_app();
        menus::activate(&mut app, command);
        assert!(app.ui.dialog.is_some(), "{command} opens a dialog");
        let lines = render(&mut app);
        assert_eq!(leaks(&lines), Vec::<&str>::new(), "English text in {command}");
    }
}

#[test]
fn the_user_can_still_choose_another_language() {
    let mut app = host_polish_app();
    app.ui.interface_language = Preference::English;
    menus::activate(&mut app, "plot");
    let lines = render(&mut app);
    assert!(lines.iter().any(|l| l == "Printer/plotter"), "Options ▸ Language ▸ English wins over the page's default: {lines:?}");
    assert!(!lines.iter().any(|l| l == "Drukarka/ploter"));
}
