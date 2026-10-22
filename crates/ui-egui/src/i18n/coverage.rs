//! Source scan for interface text that never reaches a catalog.
//!
//! Two failures make a window stay English in every language: a widget call given a bare string
//! literal (it never goes through `tl!`/`tf!`, so no catalog is consulted), and a `tl!`/`tf!` key
//! the Polish catalog has no row for. Both are caught here, so a new dialog can't ship half
//! translated. Polish is the language the web build starts in, so it is enforced; the other
//! catalogs fall back to English for gaps (see `REPORTED` in `tests.rs`).

use std::path::{Path, PathBuf};

use super::catalog;

/// Where the text of a widget call sits among its arguments.
#[derive(Clone, Copy)]
enum Arg {
    First,
    Last,
}

/// Calls whose text argument is shown to the user.
const TEXT_CALLS: &[(&str, Arg)] = &[
    (".label(", Arg::Last),
    (".button(", Arg::Last),
    (".small_button(", Arg::Last),
    (".heading(", Arg::Last),
    (".strong(", Arg::Last),
    (".checkbox(", Arg::Last),
    (".radio(", Arg::Last),
    (".radio_value(", Arg::Last),
    (".selectable_label(", Arg::Last),
    (".selectable_value(", Arg::Last),
    (".colored_label(", Arg::Last),
    (".on_hover_text(", Arg::Last),
    (".on_disabled_hover_text(", Arg::Last),
    (".hint_text(", Arg::Last),
    (".selected_text(", Arg::Last),
    (".menu_button(", Arg::First),
    (".collapsing(", Arg::First),
    ("Button::new(", Arg::Last),
    ("RichText::new(", Arg::Last),
    ("Window::new(", Arg::Last),
    ("CollapsingHeader::new(", Arg::Last),
];

/// Calls that look a literal up in the catalog.
const LOOKUP_CALLS: &[&str] = &["tl!(", "tf!(", "i18n::t("];

/// Literals that are the same in every language (formats, file types, units, product names).
const RAW_OK: &[&str] =
    &["CADCraft", "PDF", "PNG", "SVG", "DXF", "DWG", "AutoCAD", "PDMODE {v}", "{mm:.2} mm", "{w:.1} × {h:.1} mm", "{} UTC · {} KB"];

struct Source {
    path: PathBuf,
    /// The production part of the file: everything before a trailing `#[cfg(test)] mod`.
    text: String,
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            // The catalog code itself quotes its own macros in docs and tests.
            if path.file_name().is_some_and(|n| n != "i18n") {
                walk(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") && path.file_name().is_some_and(|n| n != "tests.rs") {
            out.push(path);
        }
    }
}

fn sources() -> Vec<Source> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&root, &mut files);
    files
        .into_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            let cut =
                text.match_indices("#[cfg(test)]").find(|(i, _)| text[i + 12..].trim_start().starts_with("mod ")).map_or(text.len(), |(i, _)| i);
            Some(Source { path, text: text[..cut].to_string() })
        })
        .collect()
}

/// A string literal starting at `at` (a `"`): its text and the index just past the closing quote.
fn literal(text: &str, at: usize) -> Option<(String, usize)> {
    let mut out = String::new();
    let mut chars = text.get(at + 1..)?.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Some((out, at + 1 + i + 1)),
            '\\' => match chars.next()?.1 {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                '\'' => out.push('\''),
                '\\' => out.push('\\'),
                'u' => {
                    let mut hex = String::new();
                    for (_, h) in chars.by_ref() {
                        if h == '}' {
                            break;
                        }
                        if h != '{' {
                            hex.push(h);
                        }
                    }
                    out.push(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32)?);
                }
                // A line continuation drops the newline and the next line's indentation.
                '\n' => {
                    while let Some((_, w)) = chars.clone().next() {
                        if !w.is_whitespace() {
                            break;
                        }
                        chars.next();
                    }
                }
                _ => return None,
            },
            c => out.push(c),
        }
    }
    None
}

/// The top-level arguments of the call whose `(` is at `open`, as trimmed byte ranges.
fn args(text: &str, open: usize) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let (mut depth, mut start, mut i) = (0usize, open + 1, open);
    let mut out = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let Some((_, end)) = literal(text, i) else { return out };
                i = end;
                continue;
            }
            b'\'' => {
                // A char literal ('x', '\n', '"') — a lifetime has no closing quote.
                if bytes.get(i + 2) == Some(&b'\'') {
                    i += 3;
                    continue;
                }
                if bytes.get(i + 1) == Some(&b'\\')
                    && let Some(close) = text[i + 2..].find('\'')
                {
                    i += 2 + close + 1;
                    continue;
                }
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    out.push((start, i));
                    break;
                }
            }
            b',' if depth == 1 => {
                out.push((start, i));
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.into_iter()
        .map(|(a, b)| (a + text[a..b].len() - text[a..b].trim_start().len(), b - (text[a..b].len() - text[a..b].trim_end().len())))
        .filter(|(a, b)| a < b)
        .collect()
}

/// Two letters in a row outside `{placeholders}`: something a person would read.
fn has_words(text: &str) -> bool {
    let mut outside = String::new();
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            c if depth == 0 => outside.push(c),
            _ => {}
        }
    }
    outside.as_bytes().windows(2).any(|w| w[0].is_ascii_alphabetic() && w[1].is_ascii_alphabetic())
}

fn line_of(text: &str, at: usize) -> usize {
    text[..at].bytes().filter(|b| *b == b'\n').count() + 1
}

/// Widget calls given a literal (or a `format!` of one) instead of `tl!`/`tf!`.
fn bare_literals(source: &Source) -> Vec<String> {
    let text = &source.text;
    let mut found = Vec::new();
    for (call, which) in TEXT_CALLS {
        for (at, _) in text.match_indices(call) {
            let open = at + call.len() - 1;
            let list = args(text, open);
            let Some(&(a, b)) = (match which {
                Arg::First => list.first(),
                Arg::Last => list.last(),
            }) else {
                continue;
            };
            let arg = &text[a..b];
            let shown = if arg.starts_with('"') {
                literal(text, a).filter(|(_, end)| *end == b).map(|(s, _)| s)
            } else if let Some(rest) = arg.strip_prefix("format!(") {
                rest.trim_start()
                    .starts_with('"')
                    .then(|| literal(text, b - rest.len() + (rest.len() - rest.trim_start().len())).map(|(s, _)| s))
                    .flatten()
            } else {
                None
            };
            if let Some(s) = shown
                && has_words(&s)
                && !RAW_OK.contains(&s.as_str())
            {
                found.push(format!(
                    "{}:{}: {call}{s:?}",
                    source.path.strip_prefix(env!("CARGO_MANIFEST_DIR")).unwrap_or(&source.path).display(),
                    line_of(text, at)
                ));
            }
        }
    }
    found
}

/// Every `tl!`/`tf!`/`i18n::t` key written as a literal.
fn lookup_keys(source: &Source) -> Vec<String> {
    let text = &source.text;
    let mut keys = Vec::new();
    for call in LOOKUP_CALLS {
        for (at, _) in text.match_indices(call) {
            let open = at + call.len() - 1;
            let Some(&(a, _)) = args(text, open).first() else { continue };
            if text.as_bytes().get(a) == Some(&b'"')
                && let Some((key, _)) = literal(text, a)
            {
                keys.push(key);
            }
        }
    }
    keys
}

#[test]
fn scanner_finds_bare_literals_and_ignores_translated_ones() {
    let source = |text: &str| Source { path: PathBuf::from("x.rs"), text: text.to_string() };
    let found = bare_literals(&source(
        r#"ui.label("Layout"); ui.label(crate::tl!("Layout")); ui.checkbox(&mut fit, "Fit to paper"); ui.label(""); ui.label("%");
           ui.button(format!("Plotted {n}")); ui.label(name); ui.heading(tl!("Snap, and Grid")); ui.label("ab, cd"); ui.colored_label(c, "Offsets");"#,
    ));
    let literals: Vec<_> = found.iter().map(|f| f.split_once(": ").map_or("", |p| p.1)).collect();
    assert_eq!(
        literals,
        [".label(\"Layout\"", ".label(\"ab, cd\"", ".button(\"Plotted {n}\"", ".checkbox(\"Fit to paper\"", ".colored_label(\"Offsets\""]
    );
    assert_eq!(lookup_keys(&source(r#"tl!("Open") tf!("{n} items", n = 1) i18n::t("Plot") tl!(dynamic)"#)), ["Open", "{n} items", "Plot"]);
}

#[test]
fn no_widget_is_given_bare_english_text() {
    let sources = sources();
    assert!(sources.len() > 20, "the scan found the ui-egui sources");
    let found: Vec<String> = sources.iter().flat_map(bare_literals).collect();
    assert!(found.is_empty(), "{} interface strings skip the catalog; wrap them in tl!/tf!:\n{}", found.len(), found.join("\n"));
}

/// Polish is what the web editor starts in, so every key written in the source needs a row.
#[test]
fn every_interface_string_has_a_polish_translation() {
    let Some(pl) = catalog("pl") else {
        panic!("the Polish catalog is built in");
    };
    let mut missing: Vec<String> = sources()
        .iter()
        .flat_map(lookup_keys)
        .filter(|k| has_words(k) && !pl.contains_key(&("", k.as_str())) && !RAW_OK.contains(&k.as_str()))
        .collect();
    missing.sort();
    missing.dedup();
    assert!(missing.is_empty(), "{} strings have no pl.tsv row:\n{}", missing.len(), missing.join("\n"));
}
