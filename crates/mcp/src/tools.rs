//! MCP tool definitions and dispatch.

use serde_json::{Value, json};

use crate::backend::Backend;

pub fn tool_definitions(has_ui: bool) -> Value {
    let mut tools = vec![
        json!({"name": "command_line", "description": "Type one line at the CADCraft command line, exactly as a user would at an AutoCAD-style prompt. Starts commands by name or alias (LINE, L, CIRCLE, C, PLINE, RECTANG, MOVE, TRIM, OFFSET, ZOOM…) and answers the active prompt: points `x,y`, relative `@dx,dy`, polar `@dist<angle`, distances, keywords (e.g. `c` for Close), empty text = Enter. Spaces act as Enter, so `circle 0,0 5` works in one call. Returns the new prompt and command-line output, and `error` when the line was refused.",
            "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}}),
        json!({"name": "script", "description": "Run a multi-line script (like an AutoCAD .scr file): one or more inputs per line, blank line = Enter.",
            "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}}),
        json!({"name": "execute", "description": "Run any command with JSON parameters (no prompts or dialogs). Use list_commands to find ids and parameter docs. Examples: line {points:[[0,0],[10,0]]}, circle {center:[0,0], radius:5}, layer.new {name:\"Walls\", color:\"red\", current:true}, move {handles:[\"10A\"], delta:[5,0]}, zoom {mode:\"extents\"}.",
            "inputSchema": {"type": "object", "properties": {"command": {"type": "string"}, "params": {"type": "object"}}, "required": ["command"]}}),
        json!({"name": "list_commands", "description": "List commands with ids, labels, menu paths, aliases and parameter docs. Optional filter: a substring of the command id, label or an alias.",
            "inputSchema": {"type": "object", "properties": {"filter": {"type": "string"}}}}),
        json!({"name": "inspect_drawing", "description": "Summary of the active drawing: entity counts, extents, layers, styles, blocks, layouts, selection, undo history and (optionally) entities with handles and geometry.",
            "inputSchema": {"type": "object", "properties": {"entities": {"type": "boolean"}, "limit": {"type": "integer"}}}}),
        json!({"name": "query_entities", "description": "Entities filtered by type (Line, Circle, Polyline…), layer, current selection (selected:true; active space only) and/or a window [[x0,y0],[x1,y1]] (objects whose geometry is inside or crosses it; crossing:false = entirely inside). Returns count (all matches), returned (this page) and entities; page with offset and limit (default 500).",
            "inputSchema": {"type": "object", "properties": {"type": {"type": "string"}, "layer": {"type": "string"}, "selected": {"type": "boolean"},
                "window": {"type": "array", "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}, "minItems": 2, "maxItems": 2},
                "crossing": {"type": "boolean"}, "limit": {"type": "integer", "minimum": 0}, "offset": {"type": "integer", "minimum": 0}}}}),
        json!({"name": "render", "description": "Render the active drawing to a PNG and return it as an image: fitted to the extents by default, or just the area `window` [[x0,y0],[x1,y1]] (drawing units), e.g. one sheet or one room of a large drawing. With a window and no size the image is 1600 px wide with the window's shape.",
            "inputSchema": {"type": "object", "properties": {"width": {"type": "integer", "minimum": 16}, "height": {"type": "integer", "minimum": 16}, "fit": {"type": "boolean"},
                "window": {"type": "array", "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}, "minItems": 2, "maxItems": 2},
                "path": {"type": "string"}}}}),
        json!({"name": "find_text", "description": "Read the drawing's texts: TEXT, MTEXT (formatting removed), block attributes and multileaders, each with its text, insertion point `at`, height, rotation, style, layer and handle. Filter by `contains` (any case), `layer` and/or `window` [[x0,y0],[x1,y1]] around the insertion points; page with offset and limit (default 200). Use it to find room names, sheet titles and notes without reading all the geometry.",
            "inputSchema": {"type": "object", "properties": {"contains": {"type": "string"}, "layer": {"type": "string"},
                "window": {"type": "array", "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}, "minItems": 2, "maxItems": 2},
                "limit": {"type": "integer", "minimum": 0}, "offset": {"type": "integer", "minimum": 0}}}}),
        json!({"name": "new_drawing", "description": "Create a new drawing (imperial by default, or metric).",
            "inputSchema": {"type": "object", "properties": {"metric": {"type": "boolean"}}}}),
        json!({"name": "open", "description": "Open a DXF or DWG drawing (the format is recognised from the file's contents, not its extension). `warnings` in the reply lists what reading left out (DWG reading is partial).", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}}),
        json!({"name": "save", "description": "Save the drawing (DXF or DWG by extension). A .svg, .png or .pdf path exports instead: the drawing keeps its name and unsaved changes.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}}}),
        json!({"name": "plot_sheets", "description": "Print sheets (layouts) to one PDF each in `dir`, named `NN-<layout>.pdf`. Without `sheets`/`layouts` every non-empty layout is printed in tab order. The paper (A0-A4) and scale (1:N) come from the sheet's title block texts (e.g. `FORMAT A3`, `SKALA 1:50`) unless given; `detect:false` keeps each layout's own page setup. A given or detected scale is applied to Model prints only; for layouts it is reported as `scale` for checking against the viewports. Restores the active layout. Returns `sheets`: layout, path, bytes, pages, paper, scale and where each came from (`given`, `title block`, `page setup`).",
            "inputSchema": {"type": "object", "properties": {"dir": {"type": "string"},
                "layouts": {"type": "array", "items": {"type": "string"}},
                "sheets": {"type": "array", "items": {"type": "object"}},
                "detect": {"type": "boolean"}}, "required": ["dir"]}}),
        json!({"name": "cancel", "description": "Press Escape: cancel the running command (or clear the selection).", "inputSchema": {"type": "object", "properties": {}}}),
    ];
    if has_ui {
        tools.push(json!({"name": "screenshot", "description": "Screenshot the CADCraft window (PNG).", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}}}));
        tools.push(json!({"name": "ui_inspect", "description": "UI state: panels, canvas rect, view, cursor and snap, performance.", "inputSchema": {"type": "object", "properties": {}}}));
        tools.push(json!({"name": "ui_click", "description": "Click in the drawing area at world coordinates (or screen with space:\"screen\"). Right button = Enter.", "inputSchema": {"type": "object", "properties": {"x": {"type": "number"}, "y": {"type": "number"}, "space": {"type": "string"}, "button": {"type": "string"}}, "required": ["x", "y"]}}));
    }
    json!({ "tools": tools })
}

fn text(v: &Value) -> Value {
    json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(v).unwrap_or_default()}]})
}

fn err_text(e: &str) -> Value {
    json!({"content": [{"type": "text", "text": e}], "isError": true})
}

/// Whether `v` fits the subset of JSON Schema the tool definitions use: `type`, `items`,
/// `minItems`, `maxItems` and `minimum`.
fn fits(v: &Value, schema: &Value) -> bool {
    let typed = match schema["type"].as_str() {
        Some("string") => v.is_string(),
        // JSON Schema integers include numbers such as 2.0.
        Some("integer") => v.as_f64().is_some_and(|f| f.fract() == 0.0),
        Some("number") => v.is_number(),
        Some("boolean") => v.is_boolean(),
        Some("array") => v.is_array(),
        Some("object") => v.is_object(),
        _ => true,
    };
    let len = v.as_array().map(Vec::len);
    typed
        && schema["minimum"].as_f64().is_none_or(|m| v.as_f64().is_none_or(|f| f >= m))
        && schema["minItems"].as_u64().is_none_or(|m| len.is_none_or(|n| n as u64 >= m))
        && schema["maxItems"].as_u64().is_none_or(|m| len.is_none_or(|n| n as u64 <= m))
        && (schema["items"].is_null() || v.as_array().is_none_or(|a| a.iter().all(|i| fits(i, &schema["items"]))))
}

/// Arguments checked against the tool's input schema: an object, required properties present and
/// each given property fitting its schema (`null` counts as not given). `Err` is the
/// invalid-params message.
fn check_args(name: &str, args: &Value) -> Result<Value, String> {
    let defs = tool_definitions(true);
    let schema = defs["tools"].as_array().and_then(|t| t.iter().find(|t| t["name"] == name)).map(|t| t["inputSchema"].clone());
    let Some(schema) = schema else { return Err(format!("unknown tool `{name}`")) };
    let a = match args {
        Value::Null => json!({}),
        Value::Object(_) => args.clone(),
        _ => return Err(format!("`arguments` of `{name}` must be an object")),
    };
    for key in schema["required"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        if a.get(key).is_none_or(Value::is_null) {
            return Err(format!("`{name}` needs `{key}`"));
        }
    }
    for (key, prop) in schema["properties"].as_object().into_iter().flatten() {
        if let Some(v) = a.get(key).filter(|v| !v.is_null())
            && !fits(v, prop)
        {
            return Err(format!("`{key}` of `{name}` must match {prop}"));
        }
    }
    Ok(a)
}

/// Whether a command-catalog entry's id, label or one of its aliases contains `f` (lower case).
fn command_matches(c: &Value, f: &str) -> bool {
    let has = |s: &str| s.to_ascii_lowercase().contains(f);
    ["id", "label"].iter().any(|k| c[*k].as_str().is_some_and(has))
        || c["aliases"].as_array().is_some_and(|a| a.iter().filter_map(Value::as_str).any(has))
}

/// Most sheets one `plot_sheets` call prints.
const MAX_SHEETS: usize = 200;
/// Most title block texts read per sheet.
const TITLE_TEXTS: u64 = 500;

/// The paper (`A0`..`A4`) named by a text as a whole word, any case.
pub(crate) fn paper_in(t: &str) -> Option<String> {
    let c: Vec<char> = t.chars().collect();
    (0..c.len()).find_map(|i| {
        let (a, n) = (*c.get(i)?, *c.get(i + 1)?);
        let word_start = i == 0 || c.get(i - 1).is_none_or(|p| !p.is_alphanumeric());
        let word_end = c.get(i + 2).is_none_or(|p| !p.is_alphanumeric());
        (a.eq_ignore_ascii_case(&'a') && ('0'..='4').contains(&n) && word_start && word_end).then(|| format!("A{n}"))
    })
}

/// A scale `1:N` (spaces allowed around the colon) in a text, as `1:N`.
pub(crate) fn scale_in(t: &str) -> Option<String> {
    let c: Vec<char> = t.chars().collect();
    (0..c.len()).find_map(|i| {
        if *c.get(i)? != '1' || (i > 0 && c.get(i - 1).is_some_and(|p| p.is_alphanumeric() || *p == '.' || *p == ',')) {
            return None;
        }
        let mut j = i + 1;
        while c.get(j) == Some(&' ') {
            j += 1;
        }
        if c.get(j) != Some(&':') {
            return None;
        }
        j += 1;
        while c.get(j) == Some(&' ') {
            j += 1;
        }
        let digits: String = c.iter().skip(j).take_while(|d| d.is_ascii_digit()).take(7).collect();
        let n: u32 = digits.parse().ok()?;
        (n >= 1).then(|| format!("1:{n}"))
    })
}

/// The paper and scale a sheet's texts name. A text mentioning `format`/`papier` decides the paper
/// and one mentioning `skala`/`scale`/`sk.` the scale, before any other text does.
fn title_block(texts: &[String]) -> (Option<String>, Option<String>) {
    let low: Vec<String> = texts.iter().map(|t| t.to_lowercase()).collect();
    let pick = |keys: &[&str], find: fn(&str) -> Option<String>| {
        let keyed = texts.iter().zip(&low).filter(|(_, l)| keys.iter().any(|k| l.contains(k)));
        keyed.filter_map(|(t, _)| find(t)).next().or_else(|| texts.iter().find_map(|t| find(t)))
    };
    (pick(&["format", "papier", "paper"], paper_in), pick(&["skala", "scale", "sk."], scale_in))
}

fn exec(b: &mut dyn Backend, command: &str, params: Value) -> Result<Value, String> {
    b.call("engine.execute", json!({"command": command, "params": params}))
}

/// `plot_sheets`: one PDF per sheet, paper and scale from the sheet's title block.
fn plot_sheets(b: &mut dyn Backend, a: &Value) -> Result<Value, String> {
    let dir = a.get("dir").and_then(Value::as_str).ok_or("missing `dir`")?;
    let detect = a.get("detect").and_then(Value::as_bool).unwrap_or(true);
    std::fs::create_dir_all(dir).map_err(|e| format!("{dir}: {e}"))?;
    let listed = exec(b, "layout", json!({"option": "list"}))?;
    let mut layouts: Vec<(i64, String, bool, bool)> = listed
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| {
            let name = l.get("name")?.as_str()?.to_string();
            let has_vp = l.get("viewports").and_then(Value::as_array).is_some_and(|v| !v.is_empty());
            Some((l.get("tabOrder").and_then(Value::as_i64).unwrap_or(0), name, l.get("current").and_then(Value::as_bool) == Some(true), has_vp))
        })
        .collect();
    layouts.sort();
    let was = layouts.iter().find(|l| l.2).map_or("Model".to_string(), |l| l.1.clone());
    // (layout, paper, scale, landscape) as given; the sheets named explicitly are printed even when empty.
    let mut want: Vec<(String, Option<String>, Option<String>, Option<bool>, bool)> = Vec::new();
    if let Some(sheets) = a.get("sheets").and_then(Value::as_array) {
        for sh in sheets {
            let name = sh.get("layout").and_then(Value::as_str).ok_or("each of `sheets` needs a `layout`")?;
            let s = |k: &str| sh.get(k).and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_f64().map(|f| f.to_string())));
            want.push((name.to_string(), s("paper"), s("scale"), sh.get("landscape").and_then(Value::as_bool), true));
        }
    } else if let Some(names) = a.get("layouts").and_then(Value::as_array) {
        for n in names.iter().filter_map(Value::as_str) {
            want.push((n.to_string(), None, None, None, true));
        }
    } else {
        want.extend(layouts.iter().map(|l| (l.1.clone(), None, None, None, false)));
    }
    if want.len() > MAX_SHEETS {
        return Err(format!("at most {MAX_SHEETS} sheets can be printed at once"));
    }
    let mut out = Vec::new();
    let mut failure = None;
    for (i, (name, paper, scale, landscape, explicit)) in want.iter().enumerate() {
        let model = name.eq_ignore_ascii_case("model");
        let has_vp = layouts.iter().any(|l| l.1.eq_ignore_ascii_case(name) && l.3);
        let (mut found_paper, mut found_scale) = (None, None);
        let mut texts = 0;
        if !model {
            exec(b, "layout.set", json!({"name": name}))?;
            let t = exec(b, "texts", json!({"limit": TITLE_TEXTS}))?;
            let all: Vec<String> =
                t.get("texts").and_then(Value::as_array).into_iter().flatten().filter_map(|x| x.get("text")?.as_str().map(str::to_string)).collect();
            texts = all.len();
            if detect {
                (found_paper, found_scale) = title_block(&all);
            }
        }
        if !model && !explicit && !has_vp && texts == 0 {
            out.push(json!({"layout": name, "skipped": "empty layout"}));
            continue;
        }
        let file = format!("{:02}-{}.pdf", i + 1, safe_name(name));
        let path = std::path::Path::new(dir).join(file).to_string_lossy().to_string();
        let mut q = json!({"layout": name, "path": path});
        let (paper_used, paper_src) = match (paper, &found_paper) {
            (Some(p), _) => (Some(p.clone()), "given"),
            (None, Some(p)) => (Some(p.clone()), "title block"),
            _ => (None, "page setup"),
        };
        let (scale_used, scale_src) = match (scale, &found_scale) {
            (Some(s), _) => (Some(s.clone()), "given"),
            (None, Some(s)) => (Some(s.clone()), "title block"),
            _ => (None, "page setup"),
        };
        if let Some(p) = &paper_used {
            q["paper"] = json!(p);
        }
        if let Some(l) = landscape {
            q["landscape"] = json!(l);
        }
        if let (true, Some(s)) = (model, &scale_used) {
            q["scale"] = json!(s);
        }
        match exec(b, "plot", q) {
            Ok(mut r) => {
                if let Some(o) = r.as_object_mut() {
                    o.insert("layout".into(), json!(name));
                    o.insert("paper".into(), json!({"value": paper_used, "source": paper_src}));
                    o.insert("scale".into(), json!({"value": scale_used, "source": scale_src, "applied": model && scale_used.is_some()}));
                }
                out.push(r);
            }
            Err(e) => {
                failure = Some(format!("{name}: {e}"));
                break;
            }
        }
    }
    // Leave the drawing on the layout it was on, also after a failure.
    exec(b, "layout.set", json!({"name": was}))?;
    match failure {
        Some(e) => Err(format!("{e} ({} sheet(s) printed before)", out.len())),
        None => Ok(json!({"dir": dir, "sheets": out})),
    }
}

/// A layout name made safe for a file name.
fn safe_name(name: &str) -> String {
    name.chars().map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.') { c } else { '_' }).collect()
}

/// Run a tool. `Err` is a protocol error (unknown tool, or arguments that don't fit its input
/// schema), answered with JSON-RPC invalid params; a failing tool is an `isError` result.
pub fn call_tool(b: &mut dyn Backend, name: &str, args: &Value) -> Result<Value, String> {
    let a = check_args(name, args)?;
    let r = match name {
        "command_line" => b.call("cmdline.input", a),
        "script" => b.call("cmdline.script", a),
        "execute" => b.call(
            "engine.execute",
            json!({"command": a.get("command").cloned().unwrap_or(Value::Null), "params": a.get("params").cloned().unwrap_or(json!({}))}),
        ),
        "list_commands" => b.call("engine.commands", json!({})).map(|v| {
            let f = a.get("filter").and_then(Value::as_str).map(str::to_ascii_lowercase);
            match (v, f) {
                (Value::Array(items), Some(f)) => Value::Array(items.into_iter().filter(|c| command_matches(c, &f)).collect()),
                (v, _) => v,
            }
        }),
        "inspect_drawing" => b.call("drawing.inspect", a),
        "query_entities" => b.call("engine.execute", json!({"command": "entities", "params": a})),
        "find_text" => b.call("engine.execute", json!({"command": "texts", "params": a})),
        "new_drawing" => b.call("engine.execute", json!({"command": "new", "params": a})),
        "open" => b.call("app.open", a),
        "save" => b.call("app.save", a),
        "plot_sheets" => plot_sheets(b, &a),
        "cancel" => b.call("cmdline.key", json!({"key": "escape"})),
        "screenshot" => b.call("ui.screenshot", a),
        "ui_inspect" => b.call("ui.inspect", json!({})),
        "ui_click" => b.call("ui.pointer", a),
        "render" => {
            return Ok(match b.call("ui.render", a) {
                Ok(v) => match v.get("pngBase64").and_then(Value::as_str) {
                    Some(data) => json!({"content": [{"type": "image", "data": data, "mimeType": "image/png"}]}),
                    None => text(&v),
                },
                Err(e) => err_text(&e),
            });
        }
        other => return Err(format!("unknown tool `{other}`")),
    };
    Ok(match r {
        Ok(v) => text(&v),
        Err(e) => err_text(&e),
    })
}
