//! The PDF editor workspace: a working document that tools operate on,
//! design-tool style — a Figma-like shell with page thumbnails on the
//! left (drag to reorder pages), a zoomable canvas in the middle and a
//! properties inspector on the right. Cursor tool selects/moves/edits
//! live text boxes (and converts detected PDF text to editable via
//! white-out + retype); pen and highlighter draw; text places editable
//! boxes anywhere; document operations return to the editor. Export
//! bakes everything and downloads.
//!
//! Rendering is PDF.js (bundled locally, ADR-0007); all mutation is the
//! Rust engine. Pending ink/stamps/texts are auto-baked before any
//! document operation so edits survive structural changes.

use dioxus::document::eval;
use dioxus::prelude::*;
use pz_core::{stem, InputFile, OutputFile, ToolOptions};
use pz_engine::{EditImage, PageEdit, PlacedRect, PlacedText, Stroke, TextEdit};
use serde::Deserialize;

use crate::save::{object_url, save_file};

const EDITOR_JS: Asset = asset!("/assets/editor.js");
const PDFJS: Asset = asset!("/assets/pdfjs/pdf.min.mjs");
const PDFJS_WORKER: Asset = asset!("/assets/pdfjs/pdf.worker.min.mjs");

/// Waits for editor.js (loaded via a <script> tag) before touching its API.
const WAIT_FOR_SCRIPT: &str = "for (let i = 0; i < 100 && typeof pzInit === 'undefined'; i++) { await new Promise(r => setTimeout(r, 50)); } if (typeof pzInit === 'undefined') { throw new Error('editor script did not load'); }";

/// How many document states the operation-level undo/redo keeps.
const MAX_HISTORY: usize = 8;

/// The stabilo: translucent multiply-blended ink.
const HIGHLIGHT_OPACITY: f32 = 0.4;

/// Persistent JS→Rust channel: keyboard shortcuts (Ctrl+Z / Ctrl+Shift+Z
/// fall through to document-level undo/redo when the canvas has nothing
/// to undo) and thumbnail drag-reorder notifications.
const CHANNEL_JS: &str = r#"
window.pzNotify = (m) => { try { dioxus.send(m); } catch (e) {} };
if (!window.pzKeysBound) {
  window.pzKeysBound = true;
  document.addEventListener('keydown', (e) => {
    const el = document.activeElement;
    if (el && (el.isContentEditable || el.tagName === 'INPUT' || el.tagName === 'TEXTAREA')) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
      e.preventDefault();
      const redo = e.shiftKey;
      const did = redo ? (typeof pzRedo === 'function' && pzRedo())
                       : (typeof pzUndo === 'function' && pzUndo());
      if (!did && window.pzNotify) window.pzNotify(redo ? 'op-redo' : 'op-undo');
    }
  });
}
await new Promise(() => {});
"#;

/// Feed bytes to a JS function: a blob URL on the web (cheap at any size),
/// inline base64 on native webviews where `blob:` URLs can't be minted
/// from Rust.
fn bytes_to_js_call(func: &str, prefix_args: &str, bytes: &[u8], mime: &str) -> String {
    match object_url(bytes, mime) {
        Some(url) => format!("await {func}({prefix_args}'{url}')"),
        None => {
            use base64::engine::general_purpose::STANDARD as B64;
            use base64::Engine;
            format!("await {func}B64({prefix_args}'{}')", B64.encode(bytes))
        }
    }
}

/// (Re)render the working document in the JS page view. Returns page count.
async fn open_in_js(bytes: &[u8]) -> Result<usize, String> {
    let open_call = bytes_to_js_call("pzOpen", "", bytes, "application/pdf");
    let js = format!(
        "return (async () => {{ {WAIT_FOR_SCRIPT} await pzInit('{PDFJS}', '{PDFJS_WORKER}'); return {open_call}; }})();"
    );
    match eval(&js).await {
        Ok(v) => Ok(v.as_u64().unwrap_or(0) as usize),
        Err(e) => Err(format!("could not render PDF: {e:?}")),
    }
}

/// Shape of `pzExport()`'s JSON.
#[derive(Deserialize)]
struct ExportPage {
    page: u32,
    strokes: Vec<ExportStroke>,
    images: Vec<ExportImage>,
    #[serde(default)]
    texts: Vec<ExportText>,
    #[serde(default)]
    rects: Vec<ExportRect>,
    #[serde(default)]
    redacts: Vec<ExportRedact>,
    #[serde(default)]
    edits: Vec<ExportTextEdit>,
}

#[derive(Deserialize)]
struct ExportRedact {
    rect: (f32, f32, f32, f32),
}

/// A retyped span of text that already exists in the PDF: `src` is the
/// original span's rect, `dx`/`dy` how far the box was dragged since.
#[derive(Deserialize)]
struct ExportTextEdit {
    src: (f32, f32, f32, f32),
    text: String,
    color: String,
    size: f32,
    #[serde(default)]
    bold: bool,
    x: f32,
    y: f32,
    #[serde(default)]
    dx: f32,
    #[serde(default)]
    dy: f32,
}

#[derive(Deserialize)]
struct ExportText {
    text: String,
    color: String,
    size: f32,
    #[serde(default)]
    bold: bool,
    x: f32,
    y: f32,
}

fn default_opacity() -> f32 {
    1.0
}

#[derive(Deserialize)]
struct ExportStroke {
    color: String,
    width: f32,
    #[serde(default = "default_opacity")]
    opacity: f32,
    points: Vec<(f32, f32)>,
}

#[derive(Deserialize)]
struct ExportImage {
    id: String,
    rect: (f32, f32, f32, f32),
    #[serde(default = "default_opacity")]
    opacity: f32,
}

#[derive(Deserialize)]
struct ExportRect {
    rect: (f32, f32, f32, f32),
    color: (u8, u8, u8),
}

fn hex_color(s: &str) -> (u8, u8, u8) {
    let h = s.trim_start_matches('#');
    let p = |i| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    (p(0), p(2), p(4))
}

/// Pull pending drawings/stamps/texts out of the JS layer as typed edits.
async fn pending_edits(attachments: &[(String, Vec<u8>)]) -> Result<Vec<PageEdit>, String> {
    let v = eval("return pzExport();")
        .await
        .map_err(|e| format!("could not collect drawings: {e:?}"))?;
    let pages: Vec<ExportPage> = serde_json::from_value(v).unwrap_or_default();
    Ok(pages
        .into_iter()
        .map(|p| PageEdit {
            page: p.page,
            strokes: p
                .strokes
                .into_iter()
                .map(|s| Stroke {
                    color: hex_color(&s.color),
                    width: s.width,
                    points: s.points,
                    opacity: s.opacity,
                })
                .collect(),
            images: p
                .images
                .into_iter()
                .filter_map(|im| {
                    attachments
                        .iter()
                        .find(|(id, _)| *id == im.id)
                        .map(|(_, bytes)| EditImage {
                            bytes: bytes.clone(),
                            rect: im.rect,
                            opacity: im.opacity,
                        })
                })
                .collect(),
            texts: p
                .texts
                .into_iter()
                .map(|t| PlacedText {
                    text: t.text,
                    size: t.size,
                    color: hex_color(&t.color),
                    pos: (t.x, t.y),
                    bold: t.bold,
                })
                .collect(),
            rects: p
                .rects
                .into_iter()
                .map(|r| PlacedRect {
                    rect: r.rect,
                    color: r.color,
                })
                .collect(),
            redactions: p.redacts.into_iter().map(|r| r.rect).collect(),
            text_edits: p
                .edits
                .into_iter()
                .map(|e| TextEdit {
                    src: e.src,
                    text: e.text,
                    delta: (e.dx, e.dy),
                    size: e.size,
                    color: hex_color(&e.color),
                    pos: (e.x, e.y),
                    bold: e.bold,
                })
                .collect(),
        })
        .filter(|p| {
            !p.strokes.is_empty()
                || !p.images.is_empty()
                || !p.texts.is_empty()
                || !p.rects.is_empty()
                || !p.redactions.is_empty()
                || !p.text_edits.is_empty()
        })
        .collect())
}

/// A document-level operation from the inspector.
#[derive(Clone)]
enum Op {
    Rotate(i32),
    PageNumbers,
    Watermark(String),
    Crop(u32, u32, u32, u32),
    Organize(String),
    Append(Vec<u8>),
    /// Bake + download; optionally compressed or password-protected.
    Export(ExportKind),
}

#[derive(Clone)]
enum ExportKind {
    Plain,
    Compressed,
    Protected(String),
    /// Every page as a 2x PNG (single page → .png, more → .zip).
    Images,
}

#[component]
pub fn EditorPage() -> Element {
    let mut pdf = use_signal(|| Option::<(String, Vec<u8>)>::None);
    let mut history = use_signal(Vec::<Vec<u8>>::new);
    let mut redo_stack = use_signal(Vec::<Vec<u8>>::new);
    let mut num_pages = use_signal(|| 0usize);
    let mut attachments = use_signal(Vec::<(String, Vec<u8>)>::new);
    let mut color = use_signal(|| "#1130cc".to_string());
    let mut size = use_signal(|| 3u8);
    let mut hl_color = use_signal(|| "#ffe600".to_string());
    let mut hl_size = use_signal(|| 14u8);
    let mut text_size = use_signal(|| 18u8);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(String::new);
    let mut notice = use_signal(String::new);
    // Active canvas tool, mirrored to JS.
    let mut tool = use_signal(|| "cursor");
    // Which inspector section is expanded ("" = none).
    let mut panel = use_signal(|| "");
    // Inspector on small screens (CSS shows it statically on wide ones).
    let mut inspector_open = use_signal(|| false);
    // View toggles.
    let mut ruler = use_signal(|| false);
    let mut grid = use_signal(|| false);
    // Section inputs.
    let mut wm_text = use_signal(String::new);
    let mut margin = use_signal(|| {
        [
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ]
    });
    let mut export_pw = use_signal(String::new);

    let mut set_tool = move |mode: &'static str| {
        tool.set(mode);
        let (c, s, o) = match mode {
            "highlight" => (hl_color(), u32::from(hl_size()), HIGHLIGHT_OPACITY),
            "text" => (color(), u32::from(text_size()), 1.0),
            _ => (color(), u32::from(size()), 1.0),
        };
        let js = format!("pzSetTool('{mode}', '{c}', {s}, {o});");
        spawn(async move {
            let _ = eval(&js).await;
        });
    };

    let zoom = move |action: &'static str| {
        spawn(async move {
            let _ = eval(&format!("return pzZoom('{action}');")).await;
        });
    };

    // Run a document operation: bake pending edits, apply, re-render, stay
    // in the editor. Export ops download instead of replacing the doc.
    let mut apply_op = move |op: Op| {
        busy.set(true);
        error.set(String::new());
        notice.set(String::new());
        spawn(async move {
            let result: Result<String, String> = async {
                let Some((name, bytes)) = pdf() else {
                    return Err("no document loaded".to_string());
                };

                // 1. Bake pending edits so they survive the operation.
                let edits = pending_edits(&attachments.read()).await?;
                let mut work = bytes.clone();
                if !edits.is_empty() {
                    work = pz_engine::edit_pdf(&name, &work, edits, 90)
                        .map_err(|e| e.to_string())?
                        .bytes;
                }

                let run = |slug: &'static str, files: Vec<InputFile>, opts: ToolOptions| async move {
                    crate::engine::run(slug, files, &opts).await
                };
                let doc_file = |bytes: &Vec<u8>| InputFile {
                    name: name.clone(),
                    bytes: bytes.clone(),
                };

                // 2. The operation itself.
                let (new_bytes, note): (Option<Vec<u8>>, String) = match &op {
                    Op::Rotate(angle) => {
                        let out = run(
                            "rotate-pdf",
                            vec![doc_file(&work)],
                            ToolOptions {
                                angle: *angle,
                                ..Default::default()
                            },
                        ).await?;
                        (Some(out[0].bytes.clone()), format!("Rotated {angle}°"))
                    }
                    Op::PageNumbers => {
                        let out = run(
                            "page-numbers-pdf",
                            vec![doc_file(&work)],
                            Default::default(),
                        ).await?;
                        (Some(out[0].bytes.clone()), "Page numbers added".into())
                    }
                    Op::Watermark(text) => {
                        let out = run(
                            "watermark-pdf",
                            vec![doc_file(&work)],
                            ToolOptions {
                                text: text.clone(),
                                ..Default::default()
                            },
                        ).await?;
                        (Some(out[0].bytes.clone()), "Watermark stamped".into())
                    }
                    Op::Crop(l, t, r, b) => {
                        let out = run(
                            "crop-pdf",
                            vec![doc_file(&work)],
                            ToolOptions {
                                x: *l,
                                y: *t,
                                width: *r,
                                height: *b,
                                ..Default::default()
                            },
                        ).await?;
                        (Some(out[0].bytes.clone()), "Margins cropped".into())
                    }
                    Op::Organize(spec) => {
                        let out = run(
                            "reorder-pdf",
                            vec![doc_file(&work)],
                            ToolOptions {
                                pages: spec.clone(),
                                ..Default::default()
                            },
                        ).await?;
                        (Some(out[0].bytes.clone()), "Pages reorganized".into())
                    }
                    Op::Append(other) => {
                        let out = run(
                            "merge-pdf",
                            vec![
                                doc_file(&work),
                                InputFile {
                                    name: "appended.pdf".into(),
                                    bytes: other.clone(),
                                },
                            ],
                            Default::default(),
                        ).await?;
                        (Some(out[0].bytes.clone()), "PDF appended".into())
                    }
                    Op::Export(kind) => {
                        let base = format!("{}-edited.pdf", stem(&name));
                        let out = match kind {
                            ExportKind::Plain => OutputFile {
                                name: base,
                                mime: "application/pdf",
                                bytes: work.clone(),
                            },
                            ExportKind::Compressed => {
                                run("compress-pdf", vec![doc_file(&work)], Default::default()).await?
                                    .remove(0)
                            }
                            ExportKind::Protected(pw) => run(
                                "protect-pdf",
                                vec![doc_file(&work)],
                                ToolOptions {
                                    password: pw.clone(),
                                    ..Default::default()
                                },
                            ).await?
                            .remove(0),
                            ExportKind::Images => {
                                // Rasterize the BAKED working copy: re-open
                                // it first so just-drawn edits are included,
                                // render 2x PNGs in JS, zip them in Rust.
                                open_in_js(&work).await?;
                                let v = eval("return pzExportPages(2);")
                                    .await
                                    .map_err(|e| format!("could not render pages: {e:?}"))?;
                                let pages_b64: Vec<String> =
                                    serde_json::from_value(v).map_err(|e| e.to_string())?;
                                use base64::engine::general_purpose::STANDARD as B64;
                                use base64::Engine;
                                let mut files = Vec::new();
                                for (i, b64) in pages_b64.iter().enumerate() {
                                    files.push(InputFile {
                                        name: format!("{}-page-{:02}.png", stem(&name), i + 1),
                                        bytes: B64.decode(b64).map_err(|e| e.to_string())?,
                                    });
                                }
                                match files.len() {
                                    0 => return Err("no pages to export".into()),
                                    1 => OutputFile {
                                        name: files[0].name.clone(),
                                        mime: "image/png",
                                        bytes: files.remove(0).bytes,
                                    },
                                    _ => {
                                        let mut zip =
                                            run("zip-files", files, Default::default()).await?.remove(0);
                                        zip.name = format!("{}-pages.zip", stem(&name));
                                        zip
                                    }
                                }
                            }
                        };
                        let note = match save_file(&out).map_err(|e| e.to_string())? {
                            Some(path) => format!("Saved to {path}"),
                            None => format!("Downloaded {} ✅", out.name),
                        };
                        // Export never replaces the working doc, but the
                        // bake might have: keep editing on the baked copy.
                        (Some(work.clone()), note)
                    }
                };

                // 3. Update state + re-render.
                if let Some(nb) = new_bytes {
                    if nb != bytes {
                        let mut h = history.write();
                        h.push(bytes.clone());
                        if h.len() > MAX_HISTORY {
                            h.remove(0);
                        }
                        redo_stack.write().clear();
                    }
                    let pages = open_in_js(&nb).await?;
                    num_pages.set(pages);
                    pdf.set(Some((name, nb)));
                }
                Ok(note)
            }
            .await;

            match result {
                Ok(note) => {
                    notice.set(note);
                    panel.set("");
                }
                Err(e) => error.set(e),
            }
            busy.set(false);
        });
    };

    let mut op_undo = move || {
        let Some(prev) = history.write().pop() else {
            notice.set("Nothing to undo".into());
            return;
        };
        spawn(async move {
            match open_in_js(&prev).await {
                Ok(pages) => {
                    num_pages.set(pages);
                    if let Some((name, cur)) = pdf() {
                        redo_stack.write().push(cur);
                        pdf.set(Some((name, prev)));
                    }
                    notice.set("Undid last operation".into());
                }
                Err(e) => error.set(e),
            }
        });
    };

    let mut op_redo = move || {
        let Some(next) = redo_stack.write().pop() else {
            notice.set("Nothing to redo".into());
            return;
        };
        spawn(async move {
            match open_in_js(&next).await {
                Ok(pages) => {
                    num_pages.set(pages);
                    if let Some((name, cur)) = pdf() {
                        history.write().push(cur);
                        pdf.set(Some((name, next)));
                    }
                    notice.set("Redid operation".into());
                }
                Err(e) => error.set(e),
            }
        });
    };

    // Canvas-level undo/redo first; document-level as the fallback.
    let unified = move |redo: bool| {
        spawn(async move {
            let call = if redo {
                "return pzRedo();"
            } else {
                "return pzUndo();"
            };
            let did = eval(call)
                .await
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !did {
                if redo {
                    op_redo();
                } else {
                    op_undo();
                }
            }
        });
    };

    // Keyboard shortcuts + thumbnail-reorder notifications from JS.
    use_effect(move || {
        spawn(async move {
            let mut ev = eval(CHANNEL_JS);
            while let Ok(msg) = ev.recv::<String>().await {
                if let Some(spec) = msg.strip_prefix("reorder:") {
                    apply_op(Op::Organize(spec.to_string()));
                } else if msg == "op-undo" {
                    op_undo();
                } else if msg == "op-redo" {
                    op_redo();
                }
            }
        });
    });

    let mut toggle_view = move |kind: &'static str| {
        let on = if kind == "ruler" {
            let v = !ruler();
            ruler.set(v);
            v
        } else {
            let v = !grid();
            grid.set(v);
            v
        };
        spawn(async move {
            let _ = eval(&format!(
                "setTimeout(() => {{ if (typeof pzView === 'function') pzView('{kind}', {on}); if (typeof pzDrawRulers === 'function') pzDrawRulers(); }}, 60);"
            ))
            .await;
        });
    };

    // Inspector accordion header.
    let sec = move |id: &'static str, label: &'static str| {
        rsx! {
            button {
                class: if panel() == id { "ed-sec-h open" } else { "ed-sec-h" },
                onclick: move |_| panel.set(if panel() == id { "" } else { id }),
                span { {label} }
                span { class: "ed-caret", {if panel() == id { "▾" } else { "▸" }} }
            }
        }
    };

    let doc_name = pdf
        .read()
        .as_ref()
        .map(|(n, _)| n.clone())
        .unwrap_or_else(|| "Edit PDF".to_string());
    let loaded = pdf.read().is_some();

    let wrap_class = match (loaded, ruler()) {
        (true, true) => "ed-canvas-wrap ed-rulers",
        _ => "ed-canvas-wrap",
    };

    rsx! {
        document::Script { src: EDITOR_JS }
        if let Some(seo) = pz_core::seo::seo_for("edit-pdf") {
            document::Title { "{seo.title}" }
            document::Meta { name: "description", content: seo.description }
        }

        div { class: if loaded { "ed-shell" } else { "ed-shell empty" },
            // ---- top bar ----
            div { class: "ed-top",
                div { class: "ed-title",
                    span { class: "ed-docname", "{doc_name}" }
                    if loaded {
                        span { class: "muted small", "{num_pages} page(s)" }
                    }
                }
                div { class: "ed-center",
                    div { class: "ed-modes",
                        button {
                            class: if tool() == "cursor" { "ed-mode active" } else { "ed-mode" },
                            title: "Cursor — select, move and edit text (click page text to edit it)",
                            onclick: move |_| set_tool("cursor"),
                            "✥"
                        }
                        button {
                            class: if tool() == "pan" { "ed-mode active" } else { "ed-mode" },
                            title: "Hand — drag to pan (Ctrl + scroll zooms)",
                            onclick: move |_| set_tool("pan"),
                            "✋"
                        }
                        button {
                            class: if tool() == "pen" { "ed-mode active" } else { "ed-mode" },
                            title: "Pen — draw or sign by hand",
                            onclick: move |_| set_tool("pen"),
                            "✒"
                        }
                        button {
                            class: if tool() == "highlight" { "ed-mode active" } else { "ed-mode" },
                            title: "Highlighter — translucent stabilo",
                            onclick: move |_| set_tool("highlight"),
                            "🖍"
                        }
                        button {
                            class: if tool() == "text" { "ed-mode active" } else { "ed-mode" },
                            title: "Text — click anywhere on a page to type",
                            onclick: move |_| set_tool("text"),
                            "🅰"
                        }
                        label { class: "ed-mode", r#for: "img-in", title: "Image — inserts at source size; drag, resize, set opacity", "🖼" }
                        button {
                            class: if tool() == "redact" { "ed-mode active" } else { "ed-mode" },
                            title: "Redact — drag a box; the text inside is permanently REMOVED from the file on export, not just covered",
                            onclick: move |_| {
                                set_tool("redact");
                                notice.set(
                                    "Drag a box over sensitive text. On export the text under it is permanently removed from the file — not just covered."
                                        .into(),
                                );
                            },
                            "▓"
                        }
                    }
                    div { class: "ed-group",
                        button {
                            class: "ed-icon",
                            title: "Undo (Ctrl+Z)",
                            onclick: move |_| unified(false),
                            "↶"
                        }
                        button {
                            class: "ed-icon",
                            title: "Redo (Ctrl+Shift+Z)",
                            onclick: move |_| unified(true),
                            "↷"
                        }
                    }
                    div { class: "ed-group",
                        button { class: "ed-icon", title: "Zoom out", onclick: move |_| zoom("out"), "−" }
                        span { id: "pz-zoomlvl", class: "ed-zoomlvl", "100%" }
                        button { class: "ed-icon", title: "Zoom in", onclick: move |_| zoom("in"), "+" }
                        button { class: "ed-icon", title: "Fit width", onclick: move |_| zoom("fit"), "⤢" }
                    }
                    div { class: "ed-group",
                        button {
                            class: if ruler() { "ed-icon active" } else { "ed-icon" },
                            title: "Toggle rulers (PDF points)",
                            onclick: move |_| toggle_view("ruler"),
                            "📏"
                        }
                        button {
                            class: if grid() { "ed-icon active" } else { "ed-icon" },
                            title: "Toggle grid",
                            onclick: move |_| toggle_view("grid"),
                            "⊞"
                        }
                    }
                }
                div { class: "ed-actions",
                    button {
                        class: "ed-icon ed-insp-toggle",
                        title: "Toggle inspector",
                        onclick: move |_| inspector_open.set(!inspector_open()),
                        "🎛"
                    }
                    button {
                        class: "primary small-btn",
                        disabled: !loaded,
                        onclick: move |_| {
                            panel.set("export");
                            inspector_open.set(true);
                        },
                        "Export ↓"
                    }
                }
            }
            if busy() {
                div { class: "ed-busybar" }
            }

            div { class: "ed-main",
                // ---- page thumbnails (drag to reorder) ----
                aside { class: "ed-left",
                    div { id: "pz-thumbs" }
                    if loaded {
                        label {
                            class: "ed-addpdf",
                            r#for: "append-in",
                            title: "Merge another PDF — its pages stack below the last page",
                            span { class: "ed-addpdf-plus", "＋" }
                            span { "Append PDF" }
                        }
                        p { class: "ed-thumbhint", "Drag pages to reorder" }
                    }
                }

                // ---- canvas ----
                div { class: wrap_class,
                    div { class: "ed-ruler ed-ruler-corner" }
                    canvas { id: "pz-ruler-h", class: "ed-ruler ed-ruler-h" }
                    canvas { id: "pz-ruler-v", class: "ed-ruler ed-ruler-v" }
                    div { class: "ed-canvas",
                        div { id: "pz-pages", class: if grid() { "pz-grid-on" } else { "" } }
                        if !loaded {
                            div { class: "ed-drop",
                                div { class: "panel ed-drop-card",
                                    label { class: "dropzone", r#for: "pdf-in",
                                        span { class: "dz-icon", "⬆" }
                                        span { class: "dz-label", "Choose a PDF to edit" }
                                        span { class: "dz-hint", "Files stay on this device — always." }
                                    }
                                    if busy() {
                                        p { class: "muted", "Rendering pages…" }
                                    }
                                    if !error.read().is_empty() {
                                        p { class: "error", "{error}" }
                                    }
                                }
                            }
                        }
                    }
                    // Always mounted: editor.js writes into it right after
                    // rendering, which can happen before `loaded` flips.
                    div { class: "ed-float ed-pageind",
                        span { id: "pz-pageno", "–" }
                    }
                }

                // ---- inspector ----
                aside { class: if inspector_open() { "ed-right open" } else { "ed-right" },
                    div { class: "ed-sec",
                        span { class: "ed-sec-title", "✒ Pen" }
                        div { class: "ed-row",
                            input {
                                r#type: "color",
                                aria_label: "Pen color",
                                value: "{color}",
                                oninput: move |evt| {
                                    color.set(evt.value());
                                    set_tool("pen");
                                },
                            }
                            input {
                                r#type: "range",
                                aria_label: "Pen size",
                                min: "1",
                                max: "16",
                                value: "{size}",
                                oninput: move |evt| {
                                    size.set(evt.value().parse().unwrap_or(3));
                                    set_tool("pen");
                                },
                            }
                            span { class: "muted small", "{size}px" }
                        }
                    }

                    div { class: "ed-sec",
                        span { class: "ed-sec-title", "🖍 Highlighter" }
                        div { class: "ed-row",
                            input {
                                r#type: "color",
                                aria_label: "Highlighter color",
                                value: "{hl_color}",
                                oninput: move |evt| {
                                    hl_color.set(evt.value());
                                    set_tool("highlight");
                                },
                            }
                            input {
                                r#type: "range",
                                aria_label: "Highlighter size",
                                min: "6",
                                max: "32",
                                value: "{hl_size}",
                                oninput: move |evt| {
                                    hl_size.set(evt.value().parse().unwrap_or(14));
                                    set_tool("highlight");
                                },
                            }
                            span { class: "muted small", "{hl_size}px" }
                        }
                    }

                    div { class: "ed-sec",
                        span { class: "ed-sec-title", "🅰 Text" }
                        div { class: "ed-row",
                            input {
                                r#type: "range",
                                aria_label: "Text size",
                                min: "8",
                                max: "72",
                                value: "{text_size}",
                                oninput: move |evt| {
                                    text_size.set(evt.value().parse().unwrap_or(18));
                                    if tool() == "text" {
                                        set_tool("text");
                                    }
                                },
                            }
                            span { class: "muted small", "{text_size}px" }
                        }
                        p { class: "muted small",
                            "Pick the text tool, click a page to type. Click existing "
                            "text to edit it (best on white backgrounds). Boxes stay "
                            "movable and editable until export."
                        }
                    }

                    div { class: "ed-sec",
                        {sec("rotate", "🔄 Rotate pages")}
                        if panel() == "rotate" {
                            div { class: "ed-sec-body ed-row",
                                button { class: "ghost", disabled: busy(), onclick: move |_| apply_op(Op::Rotate(90)), "90° ↻" }
                                button { class: "ghost", disabled: busy(), onclick: move |_| apply_op(Op::Rotate(180)), "180°" }
                                button { class: "ghost", disabled: busy(), onclick: move |_| apply_op(Op::Rotate(270)), "90° ↺" }
                            }
                        }
                    }

                    div { class: "ed-sec",
                        button {
                            class: "ed-sec-h",
                            disabled: busy(),
                            onclick: move |_| apply_op(Op::PageNumbers),
                            span { "🔢 Add page numbers" }
                        }
                    }

                    div { class: "ed-sec",
                        {sec("watermark", "💧 Watermark")}
                        if panel() == "watermark" {
                            div { class: "ed-sec-body",
                                input {
                                    r#type: "text",
                                    aria_label: "Watermark text",
                                    placeholder: "CONFIDENTIAL",
                                    value: "{wm_text}",
                                    oninput: move |evt| wm_text.set(evt.value()),
                                }
                                button {
                                    class: "primary small-btn",
                                    disabled: busy(),
                                    onclick: move |_| apply_op(Op::Watermark(wm_text())),
                                    "Stamp"
                                }
                            }
                        }
                    }

                    div { class: "ed-sec",
                        {sec("crop", "✂ Crop margins")}
                        if panel() == "crop" {
                            div { class: "ed-sec-body",
                                span { class: "muted small", "Trim (PDF points, 72 = 1\")" }
                                div { class: "ed-grid2",
                                    for (i , ph) in ["Left", "Top", "Right", "Bottom"].iter().enumerate() {
                                        input {
                                            r#type: "number",
                                            aria_label: "{ph} margin (points)",
                                            placeholder: *ph,
                                            value: "{margin.read()[i]}",
                                            oninput: move |evt| margin.write()[i] = evt.value(),
                                        }
                                    }
                                }
                                button {
                                    class: "primary small-btn",
                                    disabled: busy(),
                                    onclick: move |_| {
                                        let m = margin.read();
                                        let p = |i: usize| m[i].trim().parse().unwrap_or(0u32);
                                        apply_op(Op::Crop(p(0), p(1), p(2), p(3)));
                                    },
                                    "Crop"
                                }
                            }
                        }
                    }

                    div { class: "ed-sec",
                        label {
                            class: "ed-sec-h",
                            r#for: "append-in",
                            title: "Merge another PDF — its pages stack below the last page",
                            span { "➕ Append another PDF" }
                        }
                    }

                    div { class: "ed-sec",
                        {sec("export", "⬇ Export")}
                        if panel() == "export" {
                            div { class: "ed-sec-body",
                                button {
                                    class: "primary small-btn",
                                    disabled: busy(),
                                    onclick: move |_| apply_op(Op::Export(ExportKind::Plain)),
                                    "⬇ Download PDF"
                                }
                                button {
                                    class: "ghost",
                                    disabled: busy(),
                                    onclick: move |_| apply_op(Op::Export(ExportKind::Compressed)),
                                    "⬇ Compressed"
                                }
                                button {
                                    class: "ghost",
                                    disabled: busy(),
                                    onclick: move |_| apply_op(Op::Export(ExportKind::Images)),
                                    "⬇ Pages as PNG"
                                }
                                input {
                                    r#type: "password",
                                    aria_label: "Export password (optional)",
                                    placeholder: "•••••••• (optional)",
                                    value: "{export_pw}",
                                    oninput: move |evt| export_pw.set(evt.value()),
                                }
                                button {
                                    class: "ghost",
                                    disabled: busy() || export_pw.read().is_empty(),
                                    onclick: move |_| apply_op(Op::Export(ExportKind::Protected(export_pw()))),
                                    "⬇ Protected (AES-256)"
                                }
                            }
                        }
                    }

                    if !error.read().is_empty() {
                        p { class: "error", "{error}" }
                    }
                    if !notice.read().is_empty() {
                        p { class: "notice", "{notice}" }
                    }
                    p { class: "muted small ed-privacy",
                        "Rendered and edited entirely on this device — the file, your "
                        "signature and everything you type or draw never leave it."
                    }
                }
            }
        }

        // Hidden file inputs (triggered by the labels above).
        input {
            id: "pdf-in",
            class: "file-input",
            r#type: "file",
            accept: ".pdf",
            onchange: move |evt| {
                spawn(async move {
                    error.set(String::new());
                    busy.set(true);
                    if let Some(f) = evt.files().into_iter().next() {
                        match f.read_bytes().await {
                            Ok(bytes) => match open_in_js(&bytes).await {
                                Ok(pages) => {
                                    num_pages.set(pages);
                                    history.set(Vec::new());
                                    redo_stack.set(Vec::new());
                                    pdf.set(Some((f.name(), bytes.to_vec())));
                                }
                                Err(e) => error.set(e),
                            },
                            Err(e) => error.set(format!("could not read file: {e}")),
                        }
                    }
                    busy.set(false);
                });
            },
        }
        input {
            id: "img-in",
            class: "file-input",
            r#type: "file",
            accept: "image/*",
            onchange: move |evt| {
                spawn(async move {
                    if let Some(f) = evt.files().into_iter().next() {
                        if let Ok(bytes) = f.read_bytes().await {
                            let id = format!("img{}", attachments.read().len());
                            let stage_call = bytes_to_js_call(
                                "pzStageImage",
                                &format!("'{id}', "),
                                &bytes,
                                "application/octet-stream",
                            );
                            let js = format!("return (async () => {{ return {stage_call}; }})();");
                            match eval(&js).await {
                                Ok(_) => {
                                    tool.set("cursor");
                                    attachments.write().push((id, bytes.to_vec()));
                                    notice.set(
                                        "Image placed — drag to move, corner to resize, ✕ or Delete to remove."
                                            .into(),
                                    );
                                }
                                Err(e) => error.set(format!("could not load image: {e:?}")),
                            }
                        }
                    }
                });
            },
        }
        input {
            id: "append-in",
            class: "file-input",
            r#type: "file",
            accept: ".pdf",
            onchange: move |evt| {
                spawn(async move {
                    if let Some(f) = evt.files().into_iter().next() {
                        if let Ok(bytes) = f.read_bytes().await {
                            apply_op(Op::Append(bytes.to_vec()));
                        }
                    }
                });
            },
        }
    }
}
