//! CADCraft in the browser.
//!
//! Runs the same [`cadcraft_ui_egui::CadApp`] as the desktop app through eframe's web runner
//! (wgpu: WebGPU where the browser has a working hardware adapter, WebGL2 otherwise: with no adapter,
//! a software one, or none answering within seconds, `probe_webgpu` sends it to WebGL2). Build with
//! `trunk build --release` from this directory (output in `dist/web`). URL flags: `?webgl` forces
//! WebGL2, `?webgpu` trusts the browser's WebGPU whatever the probe says, `?cpu` draws the drawing
//! with egui shapes instead of the GPU canvas, `?sample` opens the sample drawing. Saving downloads
//! the drawing through the browser (`download.rs`). With `?host=parent` (inside an `<iframe>`), a
//! same-origin parent page sends the drawing to open and stores what CADCraft saves instead
//! (`host.rs`).
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(target_arch = "wasm32")]
pub mod download;
#[cfg(target_arch = "wasm32")]
mod host;

#[cfg(target_arch = "wasm32")]
mod web {
    use cadcraft_engine::Session;
    use cadcraft_ui_egui::{CadApp, Services};
    use wasm_bindgen::JsCast as _;

    const CANVAS_ID: &str = "cadcraft_canvas";
    const LOADING_ID: &str = "cadcraft_loading";

    /// The app, the files File > Open… and drag and drop read for it in the background, the
    /// page embedding it (`?host=parent`), and the browser's `prefers-reduced-motion` (it stays
    /// live, so View ▸ Accessibility ▸ Animation Effects ▸ Use System Setting follows a change).
    struct Shell(
        CadApp,
        crate::upload::Inbox,
        Option<(crate::host::Bridge, crate::host::DirtyReport, crate::host::ActiveReport)>,
        Option<web_sys::MediaQueryList>,
    );

    impl eframe::App for Shell {
        fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            self.1.take_dropped(ctx);
            self.1.drain(&mut self.0);
            self.0.system_animations = self.3.as_ref().map(|reduced| !reduced.matches());
            self.0.logic(ctx);
            if let Some((bridge, dirty_reported, active_reported)) = &self.2 {
                bridge.report_dirty(&self.0, dirty_reported);
                bridge.report_active(&self.0, active_reported);
            }
        }
        fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
            self.0.raw_input_hook(raw);
            cadcraft_ui_egui::cmdline::capture_tab(ctx, raw);
        }
        /// Preferences (interface theme and language) survive reloads; egui state is not kept.
        fn save(&mut self, storage: &mut dyn eframe::Storage) {
            storage.set_string(cadcraft_ui_egui::options::PROFILE_KEY, self.0.profile_json());
            storage.set_string(cadcraft_ui_egui::PREFS_KEY, self.0.prefs_json());
        }
        fn persist_egui_memory(&self) -> bool {
            false
        }
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            self.0.ui(ui);
            if let Some((bridge, dirty_reported, active_reported)) = &self.2 {
                bridge.report_dirty(&self.0, dirty_reported);
                bridge.report_active(&self.0, active_reported);
            }
        }
    }

    fn query() -> String {
        web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default()
    }

    fn query_param(key: &str) -> Option<String> {
        web_sys::UrlSearchParams::new_with_str(&query()).ok()?.get(key)
    }

    /// No file picker on the web: Save / Save As from the menu (and the close prompt) download the
    /// drawing under its current name; `saveas {"path": "name.dxf"}` picks another name.
    fn services() -> Services {
        Services { pick_save: Some(Box::new(|name: &str| Some(name.to_string()))), ..Services::default() }
    }

    pub fn start() {
        eframe::WebLogger::init(log::LevelFilter::Info).ok();
        cadcraft_engine::cmd::file::set_io(cadcraft_engine::cmd::file::IoHooks {
            read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
            write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
            plot: Some(|d, space, opts| cadcraft_io::plotter::plot(d, space, opts).map_err(|e| e.to_string())),
        });
        cadcraft_engine::cmd::file::set_write_as(|d, name, o| {
            cadcraft_io::write_with(d, name, &cadcraft_io::SaveOptions { version: o.version, binary: o.binary }).map_err(|e| e.to_string())
        });
        // AutoLISP's entget, entmake and entmod see entities as DXF groups.
        cadcraft_engine::cmd::file::set_entity_codec(cadcraft_engine::cmd::file::EntityCodec {
            groups: cadcraft_io::entity_groups,
            entity: cadcraft_io::entity_from_groups,
        });
        // `?host=parent`: a same-origin page framing CADCraft (such as a Nextcloud app) opens
        // drawings in it and stores what it saves. Otherwise there is no file system here: saved
        // files reach the user as browser downloads.
        let bridge = if query_param("host").as_deref() == Some("parent") { crate::host::Bridge::new() } else { None };
        match &bridge {
            Some(bridge) => {
                bridge.install();
                cadcraft_engine::cmd::file::set_deliver(crate::host::deliver);
            }
            None => cadcraft_engine::cmd::file::set_deliver(crate::download::download),
        }
        let app_bridge = bridge.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let Some(document) = web_sys::window().and_then(|w| w.document()) else { return };
            let Some(canvas) = document.get_element_by_id(CANVAS_ID).and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else {
                log::error!("missing <canvas id=\"{CANVAS_ID}\">");
                return;
            };
            let mut options = eframe::WebOptions::default();
            let gpu = crate::probe_webgpu().await;
            if crate::use_webgl(&query(), gpu) {
                log::info!("using WebGL2 (WebGPU: {gpu:?})");
                if let eframe::egui_wgpu::WgpuSetup::CreateNew(create) = &mut options.wgpu_options.wgpu_setup {
                    create.instance_descriptor.backends = eframe::wgpu::Backends::GL;
                }
            }
            let result = eframe::WebRunner::new()
                .start(
                    canvas,
                    options,
                    Box::new(move |cc| {
                        // Embedded, the parent sends the drawing: start on the Start tab, not Drawing1.
                        let session = if app_bridge.is_some() { Session::empty() } else { Session::new() };
                        let mut app = CadApp::new(session, services());
                        let browser =
                            web_sys::window().map(|w| w.navigator().languages().iter().filter_map(|v| v.as_string()).collect()).unwrap_or_default();
                        app.system_languages = crate::system_languages(query_param("lang").as_deref(), browser);
                        if let Some(profile) = cc.storage.and_then(|s| s.get_string(cadcraft_ui_egui::options::PROFILE_KEY)) {
                            app.load_profile(&profile);
                        }
                        if let Some(prefs) = cc.storage.and_then(|s| s.get_string(cadcraft_ui_egui::PREFS_KEY)) {
                            app.load_prefs(&prefs);
                        }
                        if let Some(l) = query_param("lang") {
                            if let Some(code) = cadcraft_ui_egui::i18n::locale_language(&l) {
                                if let Some(pref) = cadcraft_ui_egui::i18n::Preference::parse(code) {
                                    app.ui.interface_language = pref;
                                }
                            }
                        }
                        if let Some(s) = query_param("sheet") {
                            app.ui.mobile_sheet = match s.to_ascii_lowercase().as_str() {
                                "draw" | "rysuj" => cadcraft_ui_egui::mobile::Sheet::Draw,
                                "modify" | "zmien" => cadcraft_ui_egui::mobile::Sheet::Modify,
                                "layers" | "warstwy" => cadcraft_ui_egui::mobile::Sheet::Layers,
                                "properties" | "wlasciwosci" => cadcraft_ui_egui::mobile::Sheet::Properties,
                                "measure" | "pomiar" => cadcraft_ui_egui::mobile::Sheet::Measure,
                                "undoredo" | "cofnij" => cadcraft_ui_egui::mobile::Sheet::UndoRedo,
                                "more" | "wiecej" => cadcraft_ui_egui::mobile::Sheet::More,
                                "menu" => cadcraft_ui_egui::mobile::Sheet::Menu,
                                "cmd" | "commandline" => cadcraft_ui_egui::mobile::Sheet::CommandLine,
                                _ => cadcraft_ui_egui::mobile::Sheet::None,
                            };
                        }
                        // `?cpu`: no render state for the app, so the canvas draws with egui shapes.
                        if !crate::wants_cpu(&query())
                            && let Some(rs) = &cc.wgpu_render_state
                        {
                            // eframe's web runner has no stencil buffer: clipped layout
                            // viewports stay in the layout's display list.
                            app.set_wgpu(rs, None);
                        }
                        if query().contains("sample") {
                            let _ = app.run("ui.sample", serde_json::json!({}));
                        }
                        // No paths on the web: OPEN asks the browser's file picker, which reads the
                        // chosen file in the background (`upload.rs`), so the picker returns none.
                        let inbox = crate::upload::Inbox::new(&cc.egui_ctx);
                        let picker = inbox.clone();
                        app.services.pick_open = Some(Box::new(move || {
                            picker.pick();
                            None
                        }));
                        if let Some(bridge) = &app_bridge {
                            let bridge = bridge.clone();
                            app.on_request_open = Some(Box::new(move |id: &str| {
                                bridge.post_request_open(id);
                            }));
                        }
                        let host = app_bridge.map(|bridge| {
                            bridge.listen(inbox.clone());
                            bridge.post_ready();
                            (bridge, crate::host::DirtyReport::default(), crate::host::ActiveReport::default())
                        });
                        let reduced_motion = web_sys::window().and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten());
                        Ok(Box::new(Shell(app, inbox, host, reduced_motion)))
                    }),
                )
                .await;
            if let (Err(e), Some(bridge)) = (&result, &bridge) {
                bridge.post_failed(&format!("{e:?}"));
            }
            if let Some(el) = document.get_element_by_id(LOADING_ID) {
                match result {
                    Ok(()) => el.remove(),
                    Err(e) => {
                        el.set_inner_html(&format!("<p>CADCraft failed to start: {e:?}</p><p>A browser with WebGPU or WebGL2 is required.</p>"))
                    }
                }
            }
        });
    }
}

/// What the browser offers for WebGPU, found before eframe starts.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WebGpu {
    /// `navigator.gpu` doesn't exist.
    Missing,
    /// No adapter, or none within [`PROBE_TIMEOUT_MS`] (a driver that never answers).
    NoAdapter,
    /// A fallback (software) adapter: wgpu gets a device from it that is lost at once, which
    /// leaves a blank canvas.
    Software,
    Hardware,
}

/// How long the browser gets to answer `navigator.gpu.requestAdapter()`.
#[cfg(target_arch = "wasm32")]
const PROBE_TIMEOUT_MS: i32 = 3_000;

/// True when the URL query has `flag` as a flag (`?webgl`, `?sample&webgl`, `?webgl=1`).
#[cfg(any(target_arch = "wasm32", test))]
fn has_flag(query: &str, flag: &str) -> bool {
    query.trim_start_matches('?').split('&').any(|p| p == flag || p.strip_prefix(flag).is_some_and(|rest| rest.starts_with('=')))
}

/// Whether wgpu is held to WebGL2: `?webgl` says so, `?webgpu` says never, otherwise WebGPU is
/// used only with a working hardware adapter.
#[cfg(any(target_arch = "wasm32", test))]
fn use_webgl(query: &str, gpu: WebGpu) -> bool {
    if has_flag(query, "webgpu") {
        return false;
    }
    has_flag(query, "webgl") || gpu != WebGpu::Hardware
}

/// Ask the browser for a WebGPU adapter the way wgpu will, without hanging on a driver that
/// never answers. Never fails: anything unexpected counts as "no adapter".
#[cfg(target_arch = "wasm32")]
async fn probe_webgpu() -> WebGpu {
    use wasm_bindgen::JsCast as _;
    use wasm_bindgen::JsValue;
    use web_sys::js_sys::{Array, Function, Promise, Reflect};

    let get = |target: &JsValue, key: &str| Reflect::get(target, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED);
    let Some(window) = web_sys::window() else { return WebGpu::Missing };
    let gpu = get(window.navigator().as_ref(), "gpu");
    if gpu.is_undefined() || gpu.is_null() {
        return WebGpu::Missing;
    }
    let Ok(request) = get(&gpu, "requestAdapter").dyn_into::<Function>() else { return WebGpu::NoAdapter };
    let Ok(request) = request.call0(&gpu) else { return WebGpu::NoAdapter };
    let timeout = Promise::new(&mut |resolve: Function, _| {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, PROBE_TIMEOUT_MS);
    });
    let answer = wasm_bindgen_futures::JsFuture::from(Promise::race(&Array::of2(&request, &timeout))).await;
    let Ok(adapter) = answer else { return WebGpu::NoAdapter };
    if !adapter.is_object() {
        return WebGpu::NoAdapter; // null (none) or undefined (the timeout won)
    }
    // `isFallbackAdapter` moved from the adapter to its `info` in newer browsers.
    let fallback = [adapter.clone(), get(&adapter, "info")].iter().any(|o| o.is_object() && get(o, "isFallbackAdapter").as_bool() == Some(true));
    if fallback { WebGpu::Software } else { WebGpu::Hardware }
}

/// The languages the interface's "Auto" choice follows: the page's `?lang=pl` first (a host such as
/// CoreCAD starts the editor in its own language whatever the browser says), then the browser's.
/// A language chosen in Options still wins over both, and an unknown `lang` is ignored.
#[cfg(any(target_arch = "wasm32", test))]
fn system_languages(lang: Option<&str>, browser: Vec<String>) -> Vec<String> {
    let host = lang.and_then(cadcraft_ui_egui::i18n::locale_language).map(str::to_string);
    host.into_iter().chain(browser).collect()
}

/// True when the URL query (`?cpu&sample`) has a `cpu` flag: the CPU canvas path is requested.
#[cfg(any(target_arch = "wasm32", test))]
fn wants_cpu(query: &str) -> bool {
    query.trim_start_matches('?').split('&').any(|p| p == "cpu" || p.starts_with("cpu="))
}

#[cfg(target_arch = "wasm32")]
mod upload;

#[cfg(target_arch = "wasm32")]
fn main() {
    web::start();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("cadcraft-web only runs in the browser: build it with `trunk build --release` in apps/cadcraft-web");
}

#[cfg(test)]
mod tests {
    use super::{WebGpu, has_flag, system_languages, use_webgl, wants_cpu};
    use cadcraft_ui_egui::i18n::Preference;

    fn resolved(lang: Option<&str>, browser: &[&str]) -> &'static str {
        let browser = browser.iter().map(|s| s.to_string()).collect();
        Preference::Auto.resolve(&system_languages(lang, browser))
    }

    #[test]
    fn lang_param_sets_the_automatic_language_whatever_the_browser_says() {
        assert_eq!(resolved(Some("pl"), &["en-US", "en"]), "pl");
        assert_eq!(resolved(Some("pl-PL"), &["de"]), "pl");
        assert_eq!(resolved(Some("PL"), &[]), "pl");
        assert_eq!(resolved(Some("de"), &["pl"]), "de", "the host's language is explicit");
        assert_eq!(resolved(None, &["en-US", "pl"]), "en", "without ?lang the browser decides");
        assert_eq!(resolved(None, &["pl-PL", "en"]), "pl");
        assert_eq!(resolved(Some("xx"), &["pl"]), "pl", "an unknown ?lang is ignored");
        assert_eq!(resolved(Some(""), &[]), "en");
        // Options ▸ Language is not touched: only "Auto" follows the page.
        assert_eq!(Preference::English.resolve(&system_languages(Some("pl"), Vec::new())), "en");
    }

    #[test]
    fn webgl_is_chosen_unless_there_is_a_working_hardware_adapter() {
        for gpu in [WebGpu::Missing, WebGpu::NoAdapter, WebGpu::Software] {
            assert!(use_webgl("?sample", gpu), "{gpu:?} falls back to WebGL2");
            assert!(use_webgl("", gpu));
        }
        assert!(!use_webgl("?sample", WebGpu::Hardware));
        assert!(use_webgl("?webgl", WebGpu::Hardware), "?webgl forces it");
        assert!(use_webgl("?sample&webgl=1", WebGpu::Hardware));
        assert!(!use_webgl("?webgpu", WebGpu::Software), "?webgpu trusts the browser");
        assert!(!use_webgl("?webgl&webgpu", WebGpu::Hardware), "?webgpu wins");
        assert!(!use_webgl("?webgpux", WebGpu::Hardware) && use_webgl("?webgpux", WebGpu::Software), "only whole flags count");
    }

    #[test]
    fn flags_match_whole_names() {
        assert!(has_flag("?webgl&sample", "webgl"));
        assert!(has_flag("?sample&webgl", "webgl"));
        assert!(!has_flag("?webglx", "webgl"));
        assert!(!has_flag("?xwebgl", "webgl"));
        assert!(!has_flag("", "webgl"));
    }

    #[test]
    fn cpu_flag_is_detected() {
        assert!(wants_cpu("?cpu"));
        assert!(wants_cpu("?cpu&sample"));
        assert!(wants_cpu("?sample&cpu"));
        assert!(!wants_cpu(""));
        assert!(!wants_cpu("?sample"));
        assert!(!wants_cpu("?webgl&sample"));
        assert!(!wants_cpu("?gpucpu"));
    }
}
