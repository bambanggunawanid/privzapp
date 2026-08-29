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
    let value = value.map_err(|e| format!("could not render the PDF: {e:?}"))?;
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
