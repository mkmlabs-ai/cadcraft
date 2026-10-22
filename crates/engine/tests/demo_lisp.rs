//! Test the 3 demo LISP scripts on demo house projects.

use cadcraft_engine::Session;
use cadcraft_engine::cmd::file::{EntityCodec, set_entity_codec};

fn session() -> Session {
    set_entity_codec(EntityCodec { groups: cadcraft_io::entity_groups, entity: cadcraft_io::entity_from_groups });
    Session::new()
}

#[test]
fn test_demo_lisp_scripts() {
    let mut s = session();

    // 1. Load and test numeracja_pomieszczen.lsp
    let num_lisp = include_str!("../../../../../demo/lisp/numeracja_pomieszczen.lsp");
    s.lisp_load("numeracja_pomieszczen.lsp", num_lisp, true).unwrap();
    assert!(!s.lisp_var("C:NUMERUJ").is_nil());

    let before = s.doc().unwrap().model.len();
    s.cmdline("(wstaw-numer '(1000.0 2000.0) \"P.01\" 25.0 \"OPISY\")").unwrap();
    assert_eq!(s.doc().unwrap().model.len(), before + 1);

    // 2. Load and test tabliczka_rysunkowa.lsp
    let tab_lisp = include_str!("../../../../../demo/lisp/tabliczka_rysunkowa.lsp");
    s.lisp_load("tabliczka_rysunkowa.lsp", tab_lisp, true).unwrap();
    assert!(!s.lisp_var("C:TABLICZKA").is_nil());

    let before_tab = s.doc().unwrap().model.len();
    s.cmdline("(wstaw-tabliczke '(0.0 0.0) \"Projekt Domu\" \"Jan Kowalski\" \"1:100\" \"2026-10\" \"A-01\")").unwrap();
    assert!(s.doc().unwrap().model.len() > before_tab + 10);

    // 3. Load and test suma_powierzchni.lsp
    let area_lisp = include_str!("../../../../../demo/lisp/suma_powierzchni.lsp");
    s.lisp_load("suma_powierzchni.lsp", area_lisp, true).unwrap();
    assert!(!s.lisp_var("C:SUMAPOW").is_nil());

    // Compute polygon area of a 10x10 square (100 m2)
    let res = s.lisp_eval("(policz-pole-wieloboku '((0 0) (10 0) (10 10) (0 10)))", false, true, Vec::new()).unwrap();
    let val = res.unwrap().unwrap();
    assert_eq!(val.as_number(), Some(100.0));

    // Test on real demo house project
    let dxf_bytes = include_bytes!("../../../../../demo/projects/dom-parterowy-110/dom-parterowy-110.dxf");
    let doc = cadcraft_io::read(dxf_bytes, "dom-parterowy-110.dxf").unwrap();
    let mut s2 = session();
    s2.open_drawing(doc, "dom-parterowy-110.dxf", None);
    s2.lisp_load("suma_powierzchni.lsp", area_lisp, true).unwrap();
    let area_res = s2.lisp_eval("(suma-powierzchni-ss (ssget \"X\" '((0 . \"LWPOLYLINE\"))))", false, true, Vec::new()).unwrap();
    let area_val = area_res.unwrap().unwrap();
    let area_num = area_val.as_number().unwrap();
    assert!(area_num > 0.0, "Expected positive area for house polylines, got {area_num}");
}
