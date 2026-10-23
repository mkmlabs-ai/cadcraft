//! Host bridge recent files and active tab reporting tests (#M4).

use cadcraft_engine::Session;
use cadcraft_engine::cmd::file::{IoHooks, set_io};
use cadcraft_engine::doc::Drawing;
use cadcraft_ui_egui::{CadApp, HostRecentItem, Services, parse_host_recent_json};
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn parse_host_recent_valid_message() {
    let msg = json!({
        "type": "cadcraft:recent",
        "items": [
            {
                "id": "file_1",
                "name": "House_Plan.dwg",
                "updated": "2026-10-23T01:28:00Z",
                "size": 1048576
            },
            {
                "id": "file_2",
                "name": "Elevation.dxf",
                "updated": "2026-10-22",
                "size": 2048
            }
        ]
    });

    let items = parse_host_recent_json(&msg.to_string()).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(
        items[0],
        HostRecentItem { id: "file_1".into(), name: "House_Plan.dwg".into(), updated: "2026-10-23T01:28:00Z".into(), size: Some(1048576) }
    );
    assert_eq!(items[1], HostRecentItem { id: "file_2".into(), name: "Elevation.dxf".into(), updated: "2026-10-22".into(), size: Some(2048) });
}

#[test]
fn parse_host_recent_optional_fields_and_skips_malformed() {
    let msg = json!({
        "type": "cadcraft:recent",
        "items": [
            { "id": "f1", "name": "Minimal.dwg" },
            { "id": "", "name": "EmptyId.dwg" },
            { "id": "f2" },
            { "name": "MissingId.dwg" },
            { "id": "   ", "name": "WhitespaceId.dwg" },
            { "id": "f3", "name": "   " },
            "not an object",
            123,
            null,
            { "id": "f4", "name": "Full.dwg", "updated": "2026-10-20", "size": 512 }
        ]
    });

    let items = parse_host_recent_json(&msg.to_string()).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], HostRecentItem { id: "f1".into(), name: "Minimal.dwg".into(), updated: "".into(), size: None });
    assert_eq!(items[1], HostRecentItem { id: "f4".into(), name: "Full.dwg".into(), updated: "2026-10-20".into(), size: Some(512) });
}

#[test]
fn parse_host_recent_rejects_non_recent_message() {
    let wrong_type = json!({ "type": "cadcraft:open", "name": "test.dwg" });
    assert!(parse_host_recent_json(&wrong_type.to_string()).is_none());

    let missing_items = json!({ "type": "cadcraft:recent" });
    assert!(parse_host_recent_json(&missing_items.to_string()).is_none());

    let invalid_json = "not json at all";
    assert!(parse_host_recent_json(invalid_json).is_none());
}

#[test]
fn parse_host_recent_empty_items() {
    let msg = json!({ "type": "cadcraft:recent", "items": [] });
    let items = parse_host_recent_json(&msg.to_string()).unwrap();
    assert!(items.is_empty());
}

#[test]
fn active_name_reports_empty_for_start_and_title_for_document() {
    set_io(IoHooks { read: |_, _| Ok(Drawing::new_imperial()), write: |_, _| Ok(Vec::new()), plot: None });

    let mut app = CadApp::new(Session::empty(), Services::default());
    assert_eq!(app.active_name_for_host(), "");

    app.open_bytes("ProjectA.dwg", b"0\nSECTION\n2\nHEADER\n");
    assert_eq!(app.active_name_for_host(), "ProjectA.dwg");

    app.open_bytes("ProjectB.dwg", b"0\nSECTION\n2\nHEADER\n");
    assert_eq!(app.active_name_for_host(), "ProjectB.dwg");

    // Switching back to Start tab reports empty string
    app.ui.start_tab = true;
    assert_eq!(app.active_name_for_host(), "");

    // Switching to document tab switches active and clears start_tab
    let switched = app.switch_to_existing("ProjectA.dwg");
    assert!(switched);
    assert!(!app.ui.start_tab);
    assert_eq!(app.active_name_for_host(), "ProjectA.dwg");
}

#[test]
fn open_bytes_switches_to_existing_drawing_instead_of_duplicating() {
    set_io(IoHooks { read: |_, _| Ok(Drawing::new_imperial()), write: |_, _| Ok(Vec::new()), plot: None });

    let mut app = CadApp::new(Session::empty(), Services::default());
    app.open_bytes("Building.dwg", b"0\nSECTION\n2\nHEADER\n");
    assert_eq!(app.session.docs.len(), 1);
    assert_eq!(app.active_name_for_host(), "Building.dwg");

    app.open_bytes("Detail.dwg", b"0\nSECTION\n2\nHEADER\n");
    assert_eq!(app.session.docs.len(), 2);
    assert_eq!(app.active_name_for_host(), "Detail.dwg");

    // Re-opening "Building.dwg" switches to existing tab
    app.open_bytes("Building.dwg", b"0\nSECTION\n2\nHEADER\n");
    assert_eq!(app.session.docs.len(), 2, "must not duplicate open drawing");
    assert_eq!(app.active_name_for_host(), "Building.dwg");

    // Also works with path containing the same file name
    app.open_bytes("folder/Detail.dwg", b"0\nSECTION\n2\nHEADER\n");
    assert_eq!(app.session.docs.len(), 2, "must not duplicate open drawing with path prefix");
    assert_eq!(app.active_name_for_host(), "Detail.dwg");
}

#[test]
fn request_open_service_callback_invoked() {
    let opened_id = Rc::new(RefCell::new(None));
    let opened_clone = opened_id.clone();

    let mut app = CadApp::new(Session::empty(), Services::default());
    app.on_request_open = Some(Box::new(move |id: &str| {
        *opened_clone.borrow_mut() = Some(id.to_string());
    }));
    app.request_open("doc-uuid-456");
    assert_eq!(*opened_id.borrow(), Some("doc-uuid-456".to_string()));
}
