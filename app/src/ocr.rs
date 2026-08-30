//! Browser-side OCR through the bundled tesseract-wasm (ADR-0011).
//!
//! Recognition runs in a Web Worker; this module only moves images in
//! and text out. Images go straight to the worker; scanned PDFs are
//! rasterized first through the ADR-0009 pipeline (`crate::render`) and
//! each page's pixels are recognized in turn. `pz_engine::run` refuses
//! these slugs; the app dispatches here on `ToolPipeline::BrowserOcr`.

use dioxus::document::eval;
use dioxus::prelude::*;
use pz_core::{stem, InputFile, OutputFile, ToolOptions};

use crate::save::{object_url, revoke_object_url};

pub const OCRTOOL_JS: Asset = asset!("/assets/ocrtool.js");

/// Waits for ocrtool.js (loaded via a `<script>` tag) before calling it.
const WAIT_FOR_SCRIPT: &str = "for (let i = 0; i < 100 && typeof pzOcrInit === 'undefined'; i++) { await new Promise(r => setTimeout(r, 50)); } if (typeof pzOcrInit === 'undefined') { throw new Error('OCR engine did not load'); }";

/// The staged recognition models. The language code is spliced into a
/// URL path, so it is an allowlist, not a passthrough — adding a
/// language means staging its traineddata (scripts/fetch-ocr.sh) and
/// extending this match plus the widget.
fn safe_lang(lang: &str) -> &'static str {
    match lang {
        "ind" => "ind",
        _ => "eng",
    }
}

/// OCR one already-decoded image (given as bytes) and return its text.
async fn ocr_bytes(bytes: &[u8], lang: &str) -> Result<String, String> {
    let url = object_url(bytes, "application/octet-stream")
        .ok_or("no blob URL support on this platform")?;
    let js = format!(
        "return (async () => {{ {WAIT_FOR_SCRIPT} return pzOcrImage('{url}', '{lang}'); }})();"
    );
    let value = eval(&js).await;
    revoke_object_url(&url);
    let value = value.map_err(|e| format!("text recognition failed: {e:?}"))?;
    serde_json::from_value(value).map_err(|e| format!("unexpected OCR result: {e}"))
}

/// Run a `ToolPipeline::BrowserOcr` tool over the picked files.
pub async fn run_ocr_tool(
    files: &[InputFile],
    slug: &str,
    opts: &ToolOptions,
) -> Result<Vec<OutputFile>, String> {
    let lang = safe_lang(&opts.lang);
    match slug {
        "image-to-text" => {
            let mut out = Vec::new();
            for f in files {
                let text = ocr_bytes(&f.bytes, lang).await?;
                out.push(OutputFile {
                    name: format!("{}.txt", stem(&f.name)),
                    mime: "text/plain",
                    bytes: text.into_bytes(),
                });
            }
            Ok(out)
        }
        "ocr-pdf" => {
            let file = files.first().ok_or("pick a PDF first")?;
            // Same split as pdf-to-images: Rust checks the range syntax,
            // the renderer checks it against the real page count.
            let wanted = if opts.pages.trim().is_empty() {
                Vec::new()
            } else {
                pz_core::parse_page_ranges(&opts.pages, u32::MAX).map_err(|e| e.to_string())?
            };
            let pages =
                crate::render::render_pdf_pages(&file.bytes, opts.scale, "png", 100, &wanted)
                    .await?;
            let many = pages.len() > 1;
            let mut text = String::new();
            for (page, png) in pages {
                if many {
                    text.push_str(&format!("----- Page {page} -----\n\n"));
                }
                text.push_str(ocr_bytes(&png, lang).await?.trim_end());
                text.push_str("\n\n");
            }
            Ok(vec![OutputFile {
                name: format!("{}.txt", stem(&file.name)),
                mime: "text/plain",
                bytes: text.into_bytes(),
            }])
        }
        other => Err(format!("\"{other}\" is not an OCR tool")),
    }
}
