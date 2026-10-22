//! Tests for PDF and raster plot generation on drawings with large world coordinates
//! (such as PUWG 1992 in Poland, ~6.4 million meters).

use cadcraft_doc::{Common, Drawing, EntityKind, Layout, Line, Space, Viewport};
use cadcraft_geom::{Vec2, Vec3};
use cadcraft_io::plotter;
use serde_json::json;

fn add_line(d: &mut Drawing, a: (f64, f64), b: (f64, f64)) {
    let k = EntityKind::Line(Line { a: Vec3::new(a.0, a.1, 0.0), b: Vec3::new(b.0, b.1, 0.0) });
    d.add(&Space::Model, Common::default(), k).unwrap();
}

#[test]
fn plot_large_puwg_coordinates_window_and_extents_to_pdf() {
    let mut d = Drawing::new_metric();

    // Coordinates around Warsaw / Poland PUWG 1992 (~6.45M X, ~5.94M Y)
    let min_x = 6_453_636.0;
    let min_y = 5_947_085.0;
    let max_x = 6_456_786.0;
    let max_y = 5_949_312.0;

    // Draw boundary box and internal features
    add_line(&mut d, (min_x, min_y), (max_x, min_y));
    add_line(&mut d, (max_x, min_y), (max_x, max_y));
    add_line(&mut d, (max_x, max_y), (min_x, max_y));
    add_line(&mut d, (min_x, max_y), (min_x, min_y));
    add_line(&mut d, (min_x, min_y), (max_x, max_y));
    add_line(&mut d, (min_x + 500.0, min_y + 300.0), (min_x + 2000.0, min_y + 1500.0));

    // 1. Plot area = Window, CADCraft PDF device, A4 landscape, fit to paper, centered
    let opts_window = json!({
        "layout": "Model",
        "device": "CADCraft PDF",
        "paper": "A4",
        "landscape": true,
        "plotArea": "window",
        "window": [[min_x, min_y], [max_x, max_y]],
        "fit": true,
        "center": true,
        "output": "pdf"
    });

    let pdf_bytes = plotter::plot(&d, &Space::Model, &opts_window).expect("window PDF generation must succeed");
    assert!(!pdf_bytes.is_empty(), "PDF must not be empty");
    assert!(pdf_bytes.starts_with(b"%PDF-"), "must be valid PDF");
    let pdf_str = String::from_utf8_lossy(&pdf_bytes);
    assert!(pdf_str.contains("/MediaBox [0 0 841.89 595.276]"), "A4 landscape MediaBox must match");

    // 2. Plot area = Extents, fit to paper
    let opts_extents = json!({
        "layout": "Model",
        "device": "CADCraft PDF",
        "paper": "A4",
        "landscape": true,
        "plotArea": "extents",
        "fit": true,
        "center": true,
        "output": "pdf"
    });

    let pdf_extents = plotter::plot(&d, &Space::Model, &opts_extents).expect("extents PDF generation must succeed");
    assert!(pdf_extents.starts_with(b"%PDF-"), "must be valid PDF");

    // 3. Raster PNG preview (as used in Preview dialog)
    let opts_png = json!({
        "layout": "Model",
        "device": "None",
        "output": "png",
        "resolution": 150.0,
        "width": 297.0,
        "height": 210.0,
        "margins": [5.0, 5.0, 5.0, 5.0],
        "landscape": true,
        "plotArea": "window",
        "window": [[min_x, min_y], [max_x, max_y]],
        "fit": true,
        "center": true,
    });

    let png_bytes = plotter::plot(&d, &Space::Model, &opts_png).expect("PNG preview generation must succeed");
    assert!(png_bytes.starts_with(b"\x89PNG"), "must be valid PNG");

    // 4. Layout with viewport looking at large coordinates
    let mut layout = Layout::new("Układ1", 1);
    layout.page.paper = "A4".into();
    layout.page.width_mm = 297.0;
    layout.page.height_mm = 210.0;
    layout.page.device = "CADCraft PDF".into();

    let vp = Viewport {
        id: 2,
        center: Vec3::new(148.5, 105.0, 0.0),
        width: 250.0,
        height: 180.0,
        view_center: Vec2::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0),
        view_height: (max_y - min_y),
        on: true,
        ..Viewport::default()
    };
    layout.entities.push(cadcraft_doc::Entity { handle: cadcraft_doc::Handle(200), common: Common::default(), kind: EntityKind::Viewport(vp) });
    d.layouts.push(layout);

    let opts_layout = json!({
        "layout": "Układ1",
        "device": "CADCraft PDF",
        "paper": "A4",
        "landscape": true,
        "plotArea": "layout",
        "output": "pdf"
    });

    let pdf_layout = plotter::plot(&d, &Space::Paper("Układ1".into()), &opts_layout).expect("layout PDF must succeed");
    assert!(pdf_layout.starts_with(b"%PDF-"), "layout PDF must be valid");
}
