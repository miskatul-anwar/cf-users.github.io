//! Browser interop helpers (localStorage, clipboard, downloads, clock).
//! Implemented with a tiny inline JS module so no extra crates are needed.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
export function ls_get(k) {
    try { return localStorage.getItem(k); } catch (e) { return null; }
}
export function ls_set(k, v) {
    try { localStorage.setItem(k, v); } catch (e) {}
}
export function ls_del(k) {
    try { localStorage.removeItem(k); } catch (e) {}
}
export function epoch_ms() {
    return Date.now();
}
export function copy_text(t) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(t).catch(function () {});
    }
}
export function download_text(name, text, mime) {
    var b = new Blob([text], { type: mime || 'text/plain;charset=utf-8' });
    var u = URL.createObjectURL(b);
    var a = document.createElement('a');
    a.href = u;
    a.download = name;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(function () { URL.revokeObjectURL(u); }, 2000);
}
export function remove_element(id) {
    var el = document.getElementById(id);
    if (el && el.parentNode) el.parentNode.removeChild(el);
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = ls_get)]
    pub fn ls_get(key: &str) -> Option<String>;
    #[wasm_bindgen(js_name = ls_set)]
    pub fn ls_set(key: &str, value: &str);
    #[wasm_bindgen(js_name = ls_del)]
    pub fn ls_del(key: &str);
    #[wasm_bindgen(js_name = epoch_ms)]
    pub fn epoch_ms() -> f64;
    pub fn copy_text(text: &str);
    pub fn download_text(name: &str, text: &str, mime: &str);
    pub fn remove_element(id: &str);
}

/// Current unix time in seconds.
pub fn now_secs() -> i64 {
    (epoch_ms() / 1000.0) as i64
}

/// Download UTF-8 text (CSV/JSON) as a file.
pub fn download(filename: &str, text: &str) {
    let mime = if filename.ends_with(".json") {
        "application/json;charset=utf-8"
    } else if filename.ends_with(".csv") {
        "text/csv;charset=utf-8"
    } else {
        "text/plain;charset=utf-8"
    };
    download_text(filename, text, mime);
}

/// Escape a value for a CSV cell.
pub fn csv_cell(v: &str) -> String {
    if v.contains([',', '"', '\n']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

/// Build a CSV string from rows of pre-escaped cells.
pub fn csv(rows: Vec<Vec<String>>) -> String {
    rows.into_iter()
        .map(|r| r.join(","))
        .collect::<Vec<_>>()
        .join("\n")
}
