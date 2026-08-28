//! Per-tool SVG tile icons (app/assets/icons/<slug>.svg). Tools without
//! an SVG yet fall back to their registry emoji at every render site.
//! The SVGs are referenced as <img src> — never inlined: they share
//! internal gradient ids ("g"/"b"/"v"), which would collide if several
//! were inlined into one document.

use dioxus::prelude::*;

pub fn tool_icon(slug: &str) -> Option<Asset> {
    Some(match slug {
        "edit-pdf" => asset!("/assets/icons/edit-pdf.svg"),
        "merge-pdf" => asset!("/assets/icons/merge-pdf.svg"),
        "split-pdf" => asset!("/assets/icons/split-pdf.svg"),
        "rotate-pdf" => asset!("/assets/icons/rotate-pdf.svg"),
        "compress-pdf" => asset!("/assets/icons/compress-pdf.svg"),
        "images-to-pdf" => asset!("/assets/icons/images-to-pdf.svg"),
        "watermark-pdf" => asset!("/assets/icons/watermark-pdf.svg"),
        "reorder-pdf" => asset!("/assets/icons/reorder-pdf.svg"),
        "page-numbers-pdf" => asset!("/assets/icons/page-numbers-pdf.svg"),
        "crop-pdf" => asset!("/assets/icons/crop-pdf.svg"),
        "extract-text-pdf" => asset!("/assets/icons/extract-text-pdf.svg"),
        "repair-pdf" => asset!("/assets/icons/repair-pdf.svg"),
        "protect-pdf" => asset!("/assets/icons/protect-pdf.svg"),
        "unlock-pdf" => asset!("/assets/icons/unlock-pdf.svg"),
        _ => return None,
    })
}
