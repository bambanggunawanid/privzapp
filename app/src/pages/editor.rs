//! The PDF editor workspace: a working document that tools operate on,
//! Adobe/iLove style — draw or sign, stamp images, then rotate, number,
//! watermark, crop, reorder or append, each applying to the working copy
//! and returning to the editor. Export bakes everything and downloads
//! (optionally compressed or password-protected).
//!
//! Rendering is PDF.js (bundled locally, ADR-0007); all mutation is the
//! Rust engine. Pending ink/stamps are auto-baked before any document
//! operation so drawings survive structural changes.

use dioxus::document::eval;
use dioxus::prelude::*;
use pz_core::{stem, InputFile, OutputFile, ToolOptions};
use pz_engine::{EditImage, PageEdit, PlacedText, Stroke};
use serde::Deserialize;

use crate::save::{object_url, save_file};

const EDITOR_JS: Asset = asset!("/assets/editor.js");
const PDFJS: Asset = asset!("/assets/pdfjs/pdf.min.mjs");
const PDFJS_WORKER: Asset = asset!("/assets/pdfjs/pdf.worker.min.mjs");

/// Waits for editor.js (loaded via a <script> tag) before touching its API.
const WAIT_FOR_SCRIPT: &str = "for (let i = 0; i < 100 && typeof pzInit === 'undefined'; i++) { await new Promise(r => setTimeout(r, 50)); } if (typeof pzInit === 'undefined') { throw new Error('editor script did not load'); }";

/// How many document states the operation-level Undo keeps.
const MAX_HISTORY: usize = 8;

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
}

#[derive(Deserialize)]
struct ExportText {
    text: String,
    color: String,
    size: f32,
    x: f32,
    y: f32,
}

#[derive(Deserialize)]
struct ExportStroke {
    color: String,
    width: f32,
    points: Vec<(f32, f32)>,
}

#[derive(Deserialize)]
struct ExportImage {
    id: String,
    rect: (f32, f32, f32, f32),
}

fn hex_color(s: &str) -> (u8, u8, u8) {
    let h = s.trim_start_matches('#');
    let p = |i| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    (p(0), p(2), p(4))
}

/// Pull pending drawings/stamps out of the JS overlays as typed edits.
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
                })
                .collect(),
        })
        .filter(|p| !p.strokes.is_empty() || !p.images.is_empty() || !p.texts.is_empty())
        .collect())
}

/// A document-level operation from the toolbar.
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
}

#[component]
pub fn EditorPage() -> Element {
    let mut pdf = use_signal(|| Option::<(String, Vec<u8>)>::None);
    let mut history = use_signal(Vec::<Vec<u8>>::new);
    let mut num_pages = use_signal(|| 0usize);
    let mut attachments = use_signal(Vec::<(String, Vec<u8>)>::new);
    let mut color = use_signal(|| "#1130cc".to_string());
    let mut size = use_signal(|| 3u8);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(String::new);
    let mut notice = use_signal(String::new);
    // Which tool panel is open ("" = none).
    let mut panel = use_signal(|| "");
    // Panel inputs.
    let mut add_text = use_signal(String::new);
    let mut text_size = use_signal(|| 18u8);
    let mut wm_text = use_signal(String::new);
    let mut order_spec = use_signal(String::new);
    let mut margin = use_signal(|| {
        [
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ]
    });
    let mut export_pw = use_signal(String::new);

    let set_tool = move |mode: &'static str| {
        let js = format!("pzSetTool('{mode}', '{}', {});", color(), size());
        spawn(async move {
            let _ = eval(&js).await;
        });
    };

    // Run a document operation: bake pending ink, apply, re-render, stay
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

                // 1. Bake pending drawings so they survive the operation.
                let edits = pending_edits(&attachments.read()).await?;
                let mut work = bytes.clone();
                if !edits.is_empty() {
                    work = pz_engine::edit_pdf(&name, &work, edits, 90)
                        .map_err(|e| e.to_string())?
                        .bytes;
                }

                let run = |slug: &str, files: Vec<InputFile>, opts: ToolOptions| {
                    pz_engine::run(slug, &files, &opts).map_err(|e| e.to_string())
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
                        )?;
                        (Some(out[0].bytes.clone()), format!("Rotated {angle}°"))
                    }
                    Op::PageNumbers => {
                        let out = run(
                            "page-numbers-pdf",
                            vec![doc_file(&work)],
                            Default::default(),
                        )?;
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
                        )?;
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
                        )?;
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
                        )?;
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
                        )?;
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
                                run("compress-pdf", vec![doc_file(&work)], Default::default())?
                                    .remove(0)
                            }
                            ExportKind::Protected(pw) => run(
                                "protect-pdf",
                                vec![doc_file(&work)],
                                ToolOptions {
                                    password: pw.clone(),
                                    ..Default::default()
                                },
                            )?
                            .remove(0),
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

    let undo_op = move |_: Event<MouseData>| {
        let Some(prev) = history.write().pop() else {
            notice.set("Nothing to undo".into());
            return;
        };
        spawn(async move {
            match open_in_js(&prev).await {
                Ok(pages) => {
                    num_pages.set(pages);
                    if let Some((name, _)) = pdf() {
                        pdf.set(Some((name, prev)));
                    }
                    notice.set("Undid last operation".into());
                }
                Err(e) => error.set(e),
            }
        });
    };

    let chip = move |id: &'static str, label: &'static str| {
        rsx! {
            button {
                class: if panel() == id { "related-link chip-active" } else { "related-link" },
                onclick: move |_| panel.set(if panel() == id { "" } else { id }),
                {label}
            }
        }
    };

    rsx! {
        document::Script { src: EDITOR_JS }
        if let Some(seo) = pz_core::seo::seo_for("edit-pdf") {
            document::Title { "{seo.title}" }
            document::Meta { name: "description", content: seo.description }
        }

        section { class: "tool-head",
            div { class: "tool-icon big", "✏️" }
            div {
                h1 { "Edit PDF" }
                p { class: "muted",
                    "Sign, draw, stamp — then rotate, number, watermark, crop, "
                    "reorganize or append, all on your device."
                }
            }
        }

        if pdf.read().is_none() {
            section { class: "panel",
                label { class: "dropzone", r#for: "pdf-in",
                    span { class: "dz-icon", "⬆" }
                    span { class: "dz-label", "Choose a PDF to edit" }
                    span { class: "dz-hint", "Files stay on this device — always." }
                }
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
                if busy() {
                    p { class: "muted", "Rendering pages…" }
                }
                if !error.read().is_empty() {
                    p { class: "error", "{error}" }
                }
            }
        } else {
            section { class: "panel editor-toolbar",
                div { class: "editor-controls",
                    span { class: "muted small", "{num_pages} page(s)" }
                    button { class: "ghost", onclick: move |_| set_tool("pen"), "✒ Pen" }
                    label { class: "muted small", "Color"
                        input {
                            r#type: "color",
                            value: "{color}",
                            oninput: move |evt| {
                                color.set(evt.value());
                                set_tool("pen");
                            },
                        }
                    }
                    label { class: "muted small", "Size {size}"
                        input {
                            r#type: "range",
                            min: "1",
                            max: "16",
                            value: "{size}",
                            oninput: move |evt| {
                                size.set(evt.value().parse().unwrap_or(3));
                                set_tool("pen");
                            },
                        }
                    }
                    button {
                        class: "ghost",
                        title: "Undo last stroke or stamp",
                        onclick: move |_| {
                            spawn(async move {
                                let _ = eval("pzUndo();").await;
                            });
                        },
                        "↶ Stroke"
                    }
                    button {
                        class: "ghost",
                        title: "Undo last document operation",
                        onclick: undo_op,
                        "⎌ Operation"
                    }
                }

                div { class: "editor-controls chip-row",
                    label { class: "related-link", r#for: "img-in", "🖼 Image" }
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
                                                attachments.write().push((id, bytes.to_vec()));
                                                notice.set("Drag a rectangle on a page to place the image.".into());
                                            }
                                            Err(e) => error.set(format!("could not load image: {e:?}")),
                                        }
                                    }
                                }
                            });
                        },
                    }
                    {chip("text", "🅰 Text")}
                    {chip("rotate", "🔄 Rotate")}
                    button {
                        class: "related-link",
                        disabled: busy(),
                        onclick: move |_| apply_op(Op::PageNumbers),
                        "🔢 Page numbers"
                    }
                    {chip("watermark", "💧 Watermark")}
                    {chip("crop", "✂ Crop")}
                    {chip("organize", "🔀 Organize")}
                    label { class: "related-link", r#for: "append-in", "➕ Append PDF" }
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
                    {chip("export", "⬇ Export")}
                }

                match panel() {
                    "text" => rsx! {
                        div { class: "tool-panel",
                            textarea {
                                rows: "2",
                                placeholder: "Type your text — Enter for a new line",
                                value: "{add_text}",
                                oninput: move |evt| add_text.set(evt.value()),
                            }
                            label { class: "muted small", "Size {text_size}"
                                input {
                                    r#type: "range",
                                    min: "8",
                                    max: "72",
                                    value: "{text_size}",
                                    oninput: move |evt| text_size.set(evt.value().parse().unwrap_or(18)),
                                }
                            }
                            button {
                                class: "primary small-btn",
                                disabled: busy() || add_text.read().trim().is_empty(),
                                onclick: move |_| {
                                    // serde_json produces a safely-escaped JS string literal.
                                    let text_js = serde_json::to_string(&add_text()).unwrap_or_default();
                                    let js = format!(
                                        "pzStageText({text_js}, '{}', {});",
                                        color(),
                                        text_size()
                                    );
                                    spawn(async move {
                                        let _ = eval(&js).await;
                                        notice.set("Tap on a page where the text should go.".into());
                                        panel.set("");
                                    });
                                },
                                "Place text"
                            }
                            span { class: "muted small", "Uses the pen color. Latin characters only." }
                        }
                    },
                    "rotate" => rsx! {
                        div { class: "tool-panel",
                            span { class: "muted small", "Rotate every page:" }
                            button { class: "ghost", disabled: busy(), onclick: move |_| apply_op(Op::Rotate(90)), "90° ↻" }
                            button { class: "ghost", disabled: busy(), onclick: move |_| apply_op(Op::Rotate(180)), "180°" }
                            button { class: "ghost", disabled: busy(), onclick: move |_| apply_op(Op::Rotate(270)), "90° ↺" }
                        }
                    },
                    "watermark" => rsx! {
                        div { class: "tool-panel",
                            input {
                                r#type: "text",
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
                    },
                    "crop" => rsx! {
                        div { class: "tool-panel",
                            span { class: "muted small", "Trim margins (points, 72 = 1\")" }
                            for (i , ph) in ["Left", "Top", "Right", "Bottom"].iter().enumerate() {
                                input {
                                    r#type: "number",
                                    placeholder: *ph,
                                    value: "{margin.read()[i]}",
                                    oninput: move |evt| margin.write()[i] = evt.value(),
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
                    },
                    "organize" => rsx! {
                        div { class: "tool-panel",
                            span { class: "muted small",
                                "New page order — repeat to duplicate, omit to delete:"
                            }
                            input {
                                r#type: "text",
                                placeholder: "3,1,2",
                                value: "{order_spec}",
                                oninput: move |evt| order_spec.set(evt.value()),
                            }
                            button {
                                class: "primary small-btn",
                                disabled: busy(),
                                onclick: move |_| apply_op(Op::Organize(order_spec())),
                                "Apply order"
                            }
                        }
                    },
                    "export" => rsx! {
                        div { class: "tool-panel",
                            button {
                                class: "primary small-btn",
                                disabled: busy(),
                                onclick: move |_| apply_op(Op::Export(ExportKind::Plain)),
                                "⬇ Download"
                            }
                            button {
                                class: "ghost",
                                disabled: busy(),
                                onclick: move |_| apply_op(Op::Export(ExportKind::Compressed)),
                                "⬇ Compressed"
                            }
                            input {
                                r#type: "password",
                                placeholder: "Password (optional)",
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
                    },
                    _ => rsx! {},
                }

                if busy() {
                    p { class: "muted small", "Working…" }
                }
                if !error.read().is_empty() {
                    p { class: "error", "{error}" }
                }
                if !notice.read().is_empty() {
                    p { class: "notice", "{notice}" }
                }
            }
        }

        // PDF.js + the overlay system render into this container; Dioxus
        // declares no children here so it never touches them.
        div { id: "pz-pages" }

        section { class: "panel tool-info",
            p { class: "muted",
                "The PDF is rendered and edited entirely in your browser — the file, "
                "your signature and everything you draw never leave this device. "
                "Drawings are baked in automatically before any document operation."
            }
        }
    }
}
