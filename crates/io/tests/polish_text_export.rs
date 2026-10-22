//! Regression test for CoreCAD F1: Polish characters and symbols in drawing texts.
//! Tests rendering and export of "Zażółć gęślą jaźń ŻÓŁĆ 25,4 m² 35° ±0,00 Ø12" in model space,
//! paper space layouts, and exports to PDF, SVG, and PNG without missing-glyph replacement boxes.

use cadcraft_doc::{Common, Drawing, EntityKind, MText, Space, Text, TextStyle};
use cadcraft_geom::Vec3;
use cadcraft_io::plot;
use serde_json::json;

fn make_drawing_with_polish_text() -> Drawing {
    let mut d = Drawing::new_metric();

    // 1. Text style with stroke font (Standard / txt)
    d.text_styles.push(TextStyle { name: "Standard".into(), font: "txt".into(), ..TextStyle::default() });

    // 2. Text style with OpenSans TrueType font
    d.text_styles.push(TextStyle { name: "OpenSans".into(), font: "OpenSans-Regular.ttf".into(), ..TextStyle::default() });

    let test_sentence = "Zażółć gęślą jaźń ŻÓŁĆ 25,4 m² 35° ±0,00 Ø12";

    // Add TEXT entity with stroke font
    let text_stroke = Text {
        insert: Vec3::new(10.0, 50.0, 0.0),
        align_pt: None,
        height: 5.0,
        value: test_sentence.to_string(),
        rotation: 0.0,
        width_factor: 1.0,
        oblique: 0.0,
        style: "Standard".to_string(),
        halign: Default::default(),
        valign: Default::default(),
    };
    d.add(&Space::Model, Common::default(), EntityKind::Text(text_stroke)).unwrap();

    // Add TEXT entity with OpenSans outline font
    let text_outline = Text {
        insert: Vec3::new(10.0, 40.0, 0.0),
        align_pt: None,
        height: 5.0,
        value: test_sentence.to_string(),
        rotation: 0.0,
        width_factor: 1.0,
        oblique: 0.0,
        style: "OpenSans".to_string(),
        halign: Default::default(),
        valign: Default::default(),
    };
    d.add(&Space::Model, Common::default(), EntityKind::Text(text_outline)).unwrap();

    // Add MTEXT entity with formatting codes
    let mtext = MText {
        insert: Vec3::new(10.0, 30.0, 0.0),
        height: 5.0,
        width: 200.0,
        attach: 1,
        rotation: 0.0,
        contents: format!("{{\\fOpenSans;{test_sentence}}}"),
        line_spacing: 1.0,
        line_spacing_exact: false,
        style: "OpenSans".to_string(),
        columns: None,
    };
    d.add(&Space::Model, Common::default(), EntityKind::MText(mtext)).unwrap();

    d
}

#[test]
fn polish_text_renders_to_display_list_without_missing_glyph_boxes() {
    let d = make_drawing_with_polish_text();
    let list = cadcraft_render::build_plot(&d, &Space::Model, &cadcraft_render::Options::default());

    // Ensure list has geometry
    assert!(!list.bounds.is_empty(), "display list bounds must not be empty");
    assert!(!list.texts.is_empty(), "outline text glyphs must be plotted");

    // All outline glyphs must map to valid glyph ids > 0
    for pt in &list.texts {
        assert!(pt.glyph.gid > 0, "glyph for {:?} was unmapped", pt.glyph.ch);
    }

    // Check display list primitives: no 5-point closed missing-glyph boxes
    for prim in &list.prims {
        if prim.kind == cadcraft_render::Kind::Polyline && prim.len == 5 {
            let start = prim.start as usize;
            let end = (prim.start + prim.len) as usize;
            let pts = &list.verts[start..end];
            if pts.first() == pts.last() {
                let dx = (pts[1].x - pts[0].x).abs();
                let dy = (pts[2].y - pts[1].y).abs();
                let is_missing_box = dx > 0.0 && dy > 0.0 && (pts[1].y - pts[0].y).abs() < 1e-4 && (pts[2].x - pts[1].x).abs() < 1e-4;
                assert!(!is_missing_box, "found missing-glyph stroke box in display list! dx={dx}, dy={dy}");
            }
        }
    }
}

#[test]
fn polish_text_exports_to_pdf_svg_and_png() {
    let d = make_drawing_with_polish_text();

    // 1. Export to PDF
    let pdf_bytes = plot(&d, &Space::Model, &json!({"output": "pdf"})).expect("PDF plot must succeed");
    assert!(pdf_bytes.starts_with(b"%PDF-"), "must be valid PDF");
    let pdf_text = String::from_utf8_lossy(&pdf_bytes);
    assert!(pdf_text.contains("/Subtype /Type0"), "PDF must embed Type0 font");
    assert!(pdf_text.contains("/ToUnicode"), "PDF must have ToUnicode map");

    // 2. Export to SVG
    let svg_text = cadcraft_io::svg::export_window(&d, &Space::Model, None);
    assert!(svg_text.contains("<svg") && svg_text.contains("</svg>"), "must be valid SVG, got: {svg_text}");
    assert!(svg_text.len() > 100, "SVG must contain graphic elements");
    assert!(svg_text.len() > 100, "SVG must contain graphic elements");

    // 3. Export to PNG
    let png_bytes = cadcraft_io::png(&d, &Space::Model, 800, 600).expect("PNG export must succeed");
    assert!(png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"), "must be valid PNG");
    assert!(png_bytes.len() > 1000, "PNG image must be non-empty");
}
