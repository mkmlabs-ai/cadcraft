use std::path::Path;

use cadcraft_engine::Session;
use cadcraft_engine::cmd::file::{IoHooks, base64_decode, set_io};
use serde_json::json;

#[test]
fn demo_projects_layouts_plot_to_pdf_without_manual_plotter_change() {
    set_io(IoHooks {
        read: |_, _| Err("no reader in this test".into()),
        write: |_, _| Err("no writer in this test".into()),
        plot: Some(|doc, space, opts| cadcraft_io::plotter::plot(doc, space, opts).map_err(|e| e.to_string())),
    });

    let demo_dirs = ["dom-parterowy-110", "dom-z-poddaszem-140", "nowoczesna-stodola-120", "blizniak-2x105"];

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut tested_count = 0;
    for name in demo_dirs {
        let path = manifest_dir.join(format!("../../../../demo/projects/{name}/{name}.dxf"));
        if !path.exists() {
            continue;
        }
        let bytes = std::fs::read(&path).expect("read demo dxf");
        let doc = cadcraft_io::read_dxf(&bytes).expect("parse demo dxf");

        for layout_num in 1..=3 {
            let prefix = format!("A-0{layout_num}");
            let Some(layout) = doc.layouts.iter().find(|l| l.name.starts_with(&prefix)) else {
                continue;
            };
            let layout_name = layout.name.clone();

            let mut s = Session::new();
            s.open_drawing(doc.clone(), name, None);

            // Test 1: plot layout without device param (must succeed and generate PDF)
            let res = s.execute("plot", &json!({ "layout": layout_name })).unwrap_or_else(|e| panic!("plot {name} {layout_name} failed: {e}"));
            assert!(res["bytes"].as_u64().unwrap_or_default() > 0, "generated PDF bytes: {res}");
            let data_b64 = res["data"].as_str().expect("base64 pdf data");
            let pdf_bytes = base64_decode(data_b64).expect("decode base64");
            assert!(pdf_bytes.starts_with(b"%PDF"), "must be valid PDF");

            // Test 2: exportpdf layout without device param (must succeed and generate PDF)
            let res_exp =
                s.execute("exportpdf", &json!({ "layout": layout_name })).unwrap_or_else(|e| panic!("exportpdf {name} {layout_name} failed: {e}"));
            assert!(res_exp["bytes"].as_u64().unwrap_or_default() > 0, "generated PDF bytes: {res_exp}");
            let exp_b64 = res_exp["data"].as_str().expect("base64 pdf data");
            let exp_bytes = base64_decode(exp_b64).expect("decode base64");
            assert!(exp_bytes.starts_with(b"%PDF"), "must be valid PDF");
            tested_count += 1;
        }
    }
    // Verify that at least some layouts were found and tested if files exist
    assert!(tested_count >= 12, "expected 12 layouts across 4 demo houses, tested {tested_count}");
}

#[test]
fn synthetic_dwg_to_pdf_pc3_layout_plots_to_pdf() {
    set_io(IoHooks {
        read: |_, _| Err("no reader in this test".into()),
        write: |_, _| Err("no writer in this test".into()),
        plot: Some(|doc, space, opts| cadcraft_io::plotter::plot(doc, space, opts).map_err(|e| e.to_string())),
    });

    let mut doc = cadcraft_doc::Drawing::new_metric();
    let mut l = cadcraft_doc::Layout::new("A-01", 1);
    l.page.device = "DWG to PDF.pc3".into();
    doc.layouts.push(l);

    let mut s = Session::new();
    s.open_drawing(doc, "Test", None);

    let res = s.execute("plot", &json!({ "layout": "A-01" })).expect("plot must succeed");
    let pdf_bytes = base64_decode(res["data"].as_str().unwrap()).unwrap();
    assert!(pdf_bytes.starts_with(b"%PDF"));

    let res_exp = s.execute("exportpdf", &json!({ "layout": "A-01" })).expect("exportpdf must succeed");
    let exp_bytes = base64_decode(res_exp["data"].as_str().unwrap()).unwrap();
    assert!(exp_bytes.starts_with(b"%PDF"));
}
