//! Browser-side PDF page rasterization (ADR-0009).
//!
//! The one thing the pure-Rust engine cannot do: turn a PDF page into
//! pixels. `lopdf` parses PDFs but doesn't render them, and every real
//! rasterizer (pdfium, mupdf) is C. PDF.js is already bundled for the
//! editor (ADR-0007), so the PDF to Image tool borrows it through a
//! minimal module (`assets/pdfrender.js`) that renders and nothing else —
//! the engine still owns every byte that gets written.
//!
//! Web and desktop/mobile both run a WebView, so this works everywhere the
//! app has one; there is no headless path, which is why the tool is marked
//! `ToolPipeline::BrowserRender` and `pz_engine::run` refuses it.

use dioxus::document::eval;
use dioxus::prelude::*;

use pz_core::{stem, InputFile, OutputFile, ToolOptions};

use crate::save::{object_url, revoke_object_url};

pub const PDFRENDER_JS: Asset = asset!("/assets/pdfrender.js");
const PDFJS: Asset = asset!("/assets/pdfjs/pdf.min.mjs");
const PDFJS_WORKER: Asset = asset!("/assets/pdfjs/pdf.worker.min.mjs");

/// Waits for pdfrender.js (loaded via a `<script>` tag) before calling it.
const WAIT_FOR_SCRIPT: &str = "for (let i = 0; i < 100 && typeof pzRenderInit === 'undefined'; i++) { await new Promise(r => setTimeout(r, 50)); } if (typeof pzRenderInit === 'undefined') { throw new Error('page renderer did not load'); }";

/// Turn a raw eval failure into something worth showing a person.
///
/// The JS side rethrows `PZ_ENCRYPTED` for a password-protected
/// document (see editor.js / pdfrender.js); everything else would
/// otherwise reach the user as the `Debug` formatting of an eval error,
/// wrapping a JSON dump of a JS exception — which tells them nothing.
pub fn render_error(raw: &str) -> String {
    // One line each: the catalog is keyed by the exact English text, so a
    // wrapped literal would never match its translation.
    if raw.contains("PZ_ENCRYPTED") {
        return crate::tr(
            "That PDF is password-protected, so its pages can't be opened. Remove the password first with the Unlock PDF tool (you'll need the password), then try again.",
        );
    }
    // Anything else: keep the JS side's own message if it wrote one —
    // "page 9 is out of range (1-2)" is far more useful than a generic
    // apology, and swallowing it was a regression the tests caught.
    if let Some(msg) = js_error_message(raw) {
        return msg;
    }
    crate::tr("That PDF could not be opened — it may be damaged. Try the Repair PDF tool first.")
}

/// Pull the human-readable message out of the JS exception that an eval
/// failure wraps. Two shapes occur in practice, both pinned by tests:
///
/// - a thrown `Error`  → `JsValue(Error: <message>\n<stack>)`
/// - a PDF.js exception → `JsValue(Object({\"message\":\"<message>\", …}))`
///
/// The Debug formatting nests escaped JSON and escaped newlines, so this
/// matches on those literal sequences rather than parsing.
fn js_error_message(raw: &str) -> Option<String> {
    let msg = if let Some(i) = raw.find("JsValue(Error: ") {
        let rest = &raw[i + "JsValue(Error: ".len()..];
        // Debug escapes the newline before the stack trace.
        rest.split("\\n").next().unwrap_or(rest)
    } else {
        let key = "\\\"message\\\":\\\"";
        let start = raw.find(key)? + key.len();
        let rest = &raw[start..];
        let end = rest.find("\\\"")?;
        &rest[..end]
    }
    .trim();

    // Only surface something that reads like a sentence for a person: no
    // stack frames, no internal markers, nothing absurdly long.
    if msg.is_empty()
        || msg.len() > 200
        || msg.contains("    at ")
        || msg.contains("undefined")
        || msg.starts_with("PZ_")
    {
        return None;
    }
    Some(msg.to_string())
}

#[cfg(test)]
mod tests {
    use super::js_error_message;

    // Captured from the real app, not invented — the two shapes a failed
    // eval actually produces.
    const THROWN_ERROR: &str = r#"Communication("Failed to await result - JsValue(Error: page 9 is out of range (1-2)\nError: page 9 is out of range (1-2)\n    at pzRenderDoc (http://127.0.0.1/assets/pdfrender.js:1:617)\n    at async eval (<anonymous>:5:358))")"#;
    const PDFJS_OBJECT: &str = r#"Communication("Failed to await result - JsValue(Object({\"message\":\"No password given\",\"name\":\"PasswordException\",\"code\":1}))")"#;

    #[test]
    fn extracts_a_thrown_error_message_without_the_stack() {
        assert_eq!(
            js_error_message(THROWN_ERROR).as_deref(),
            Some("page 9 is out of range (1-2)")
        );
    }

    #[test]
    fn extracts_a_pdfjs_exception_message() {
        assert_eq!(
            js_error_message(PDFJS_OBJECT).as_deref(),
            Some("No password given")
        );
    }

    #[test]
    fn refuses_things_that_are_not_worth_showing() {
        assert_eq!(js_error_message("Communication(\"nothing useful\")"), None);
        assert_eq!(
            js_error_message("JsValue(Error: PZ_ENCRYPTED\\n at x)"),
            None
        );
    }
}

/// One rendered page, as `pdfrender.js` hands it back.
#[derive(serde::Deserialize)]
struct RenderedPage {
    page: u32,
    data: String,
}

/// The image MIME type and file extension for a raster format name.
pub fn raster_mime(format: &str) -> (&'static str, &'static str) {
    match format {
        "jpg" | "jpeg" => ("image/jpeg", "jpg"),
        "webp" => ("image/webp", "webp"),
        _ => ("image/png", "png"),
    }
}

/// Render `pages` (1-based; empty = all) of a PDF to images.
///
/// Returns `(page number, encoded bytes)` in the order rendered. `quality`
/// is 1..=100 and only affects the lossy formats.
pub async fn render_pdf_pages(
    bytes: &[u8],
    scale: u32,
    format: &str,
    quality: u8,
    pages: &[u32],
) -> Result<Vec<(u32, Vec<u8>)>, String> {
    let (mime, _) = raster_mime(format);
    let scale = scale.clamp(1, 4);
    let quality = (quality.clamp(1, 100) as f32) / 100.0;
    let list = pages
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",");

    // Prefer a blob: URL — pushing a multi-MB base64 string through an
    // eval body is the slow path, kept for builds without blob support.
    let url = object_url(bytes, "application/pdf");
    let call = match &url {
        Some(u) => format!("await pzRenderUrl('{u}', {scale}, '{mime}', {quality}, [{list}])"),
        None => {
            use base64::engine::general_purpose::STANDARD as B64;
            use base64::Engine;
            format!(
                "await pzRenderUrlB64('{}', {scale}, '{mime}', {quality}, [{list}])",
                B64.encode(bytes)
            )
        }
    };
    let js = format!(
        "return (async () => {{ {WAIT_FOR_SCRIPT} await pzRenderInit('{PDFJS}', '{PDFJS_WORKER}'); return {call}; }})();"
    );

    let value = eval(&js).await;
    if let Some(u) = &url {
        revoke_object_url(u);
    }
    let value = value.map_err(|e| render_error(&format!("{e:?}")))?;
    let rendered: Vec<RenderedPage> =
        serde_json::from_value(value).map_err(|e| format!("unexpected render result: {e}"))?;
    if rendered.is_empty() {
        return Err("that PDF has no pages to render".to_string());
    }

    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    rendered
        .into_iter()
        .map(|p| {
            B64.decode(&p.data)
                .map(|bytes| (p.page, bytes))
                .map_err(|e| format!("could not decode page {}: {e}", p.page))
        })
        .collect()
}

/// The PDF to Image pipeline: rasterize in the browser, package in the
/// engine. A single page comes back as the image itself; several are
/// zipped, which is the same shape the favicon pack and the editor's
/// "Pages as PNG" already use.
pub async fn pdf_to_images(
    file: &InputFile,
    opts: &ToolOptions,
) -> Result<Vec<OutputFile>, String> {
    // Syntax check only: the real bound ("page 9 of 4") needs the page
    // count, which nobody knows until the document is open, so the JS side
    // validates against the actual total and reports the range.
    let wanted = if opts.pages.trim().is_empty() {
        Vec::new()
    } else {
        pz_core::parse_page_ranges(&opts.pages, u32::MAX).map_err(|e| e.to_string())?
    };

    let (mime, ext) = raster_mime(&opts.format);
    let rendered =
        render_pdf_pages(&file.bytes, opts.scale, &opts.format, opts.quality, &wanted).await?;

    let base = stem(&file.name);
    let mut images: Vec<InputFile> = rendered
        .into_iter()
        .map(|(page, bytes)| InputFile {
            name: format!("{base}-page-{page:02}.{ext}"),
            bytes,
        })
        .collect();

    if images.len() == 1 {
        let one = images.remove(0);
        return Ok(vec![OutputFile {
            name: one.name,
            mime,
            bytes: one.bytes,
        }]);
    }
    let mut zip = crate::engine::run("zip-files", images, &ToolOptions::default())
        .await?
        .remove(0);
    zip.name = format!("{base}-pages.zip");
    Ok(vec![zip])
}
