//! Cost of the display list and GPU vertex data rebuilds that pan and zoom trigger, on real files.
//!
//! The web build has no worker threads, so every rebuild the view asks for (a new zoom band, a
//! pan out of the built area) is a frame that long. This measures those rebuilds the way the canvas
//! makes them (`ensure_list`), without a GPU. Ignored by default (it needs big files):
//!
//! `CADCRAFT_PERF_FILES=a.dxf:b.dwg cargo test --release -p cadcraft --test perf_frames -- --ignored --nocapture`

use std::time::Instant;

use cadcraft_color::Rgb;
use cadcraft_engine::Session;
use cadcraft_engine::doc::Space;
use cadcraft_engine::geom::{Bounds2, Vec2};

const W: f64 = 1920.0;
const H: f64 = 1080.0;
const BG: Rgb = Rgb(0x22, 0x28, 0x2f);

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// Area the canvas builds for: the view with half its larger side of margin all round.
fn window(center: Vec2, height: f64) -> Bounds2 {
    let (w, h) = (height * W / H, height);
    let view = Bounds2::new(Vec2::new(center.x - w / 2.0, center.y - h / 2.0), Vec2::new(center.x + w / 2.0, center.y + h / 2.0));
    view.expand(view.width().max(view.height()) * 0.5)
}

struct Cost {
    build: f64,
    mesh: f64,
    prims: usize,
    lines: usize,
    tris: usize,
    quads: usize,
}

/// One rebuild at `height` centred on `center`, as `ensure_list` does it.
fn rebuild(doc: &cadcraft_engine::doc::Drawing, center: Vec2, height: f64, windowed: bool) -> Cost {
    let px = height / H;
    let band = px.max(1e-300).log2().floor() as i32;
    let opts = cadcraft_render::Options {
        tolerance: 2f64.powi(band) * 0.5,
        min_dash: 2f64.powi(band) * 2.0,
        text: true,
        fill: true,
        view: windowed.then(|| window(center, height)),
        ..Default::default()
    };
    let t = Instant::now();
    let list = cadcraft_render::build(doc, &Space::Model, &opts);
    let build = ms(t);
    let t = Instant::now();
    let origin = cadcraft_ui_egui::gpu::choose_origin(&list.bounds, center, height);
    let mesh = cadcraft_ui_egui::gpu::build_mesh(&list, origin, BG, false);
    Cost { build, mesh: ms(t), prims: list.prims.len(), lines: mesh.line_vertices(), tris: mesh.tri_vertices(), quads: mesh.quad_instances() }
}

fn row(label: &str, c: &Cost) {
    println!(
        "  {label:<34} build {:>8.1} ms  mesh {:>8.1} ms  total {:>8.1} ms   prims {:>7}  lineV {:>8}  triV {:>8}  quads {:>7}",
        c.build,
        c.mesh,
        c.build + c.mesh,
        c.prims,
        c.lines,
        c.tris,
        c.quads
    );
}

#[test]
#[ignore = "needs CADCRAFT_PERF_FILES"]
fn rebuild_cost_on_real_files() {
    let files = std::env::var("CADCRAFT_PERF_FILES").unwrap_or_default();
    for path in files.split(':').filter(|p| !p.is_empty()) {
        let name = path.rsplit('/').next().unwrap_or(path);
        let Ok(bytes) = std::fs::read(path) else { panic!("cannot read {path}") };
        let t = Instant::now();
        let doc = cadcraft_io::read(&bytes, name).expect("read");
        let load = ms(t);
        let t = Instant::now();
        let exact = doc.extents(&Space::Model);
        let t_ext = ms(t);
        let t = Instant::now();
        let content = doc.content_extents(&Space::Model);
        let t_content = ms(t);
        println!(
            "\n== {name}: {} objects, load {load:.0} ms, extents {t_ext:.1} ms, content extents {t_content:.1} ms (exact {:.0}x{:.0}, content {:.0}x{:.0})",
            doc.entity_count(),
            exact.width(),
            exact.height(),
            content.width(),
            content.height()
        );
        let _ = &mut Session::empty();
        let height = content.height().max(content.width() * H / W).max(1e-6) * 1.05;
        let center = content.center();
        row("first render, whole view", &rebuild(&doc, center, height, true));
        row("first render, no window", &rebuild(&doc, center, height, false));
        let z = rebuild(&doc, center, height / 2.0, true);
        row("zoom x2 (windowed)", &z);
        row("zoom x10 (windowed)", &rebuild(&doc, center, height / 10.0, true));
        row("zoom x100 (windowed)", &rebuild(&doc, center, height / 100.0, true));
        let pan = Vec2::new(center.x + height * 0.4, center.y + height * 0.1);
        row("pan out of window at x10", &rebuild(&doc, pan, height / 10.0, true));
    }
}
