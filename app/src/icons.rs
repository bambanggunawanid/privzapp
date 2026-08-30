//! Per-tool SVG tile icons (app/assets/icons/<slug>.svg). Every tool in
//! the registry has one; a tool added without an icon falls back to its
//! registry emoji at every render site rather than rendering nothing.
//! The SVGs are referenced as <img src> — never inlined: they share
//! internal gradient ids ("g"/"b"/"m"/"v"), which would collide if
//! several were inlined into one document.

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
        "pdf-to-images" => asset!("/assets/icons/pdf-to-images.svg"),
        "repair-pdf" => asset!("/assets/icons/repair-pdf.svg"),
        "protect-pdf" => asset!("/assets/icons/protect-pdf.svg"),
        "unlock-pdf" => asset!("/assets/icons/unlock-pdf.svg"),
        "convert-img" => asset!("/assets/icons/convert-img.svg"),
        "resize-img" => asset!("/assets/icons/resize-img.svg"),
        "compress-img" => asset!("/assets/icons/compress-img.svg"),
        "rotate-img" => asset!("/assets/icons/rotate-img.svg"),
        "flip-img" => asset!("/assets/icons/flip-img.svg"),
        "upscale-img" => asset!("/assets/icons/upscale-img.svg"),
        "grayscale-img" => asset!("/assets/icons/grayscale-img.svg"),
        "blur-img" => asset!("/assets/icons/blur-img.svg"),
        "watermark-img" => asset!("/assets/icons/watermark-img.svg"),
        "strip-exif" => asset!("/assets/icons/strip-exif.svg"),
        "crop-img" => asset!("/assets/icons/crop-img.svg"),
        "favicon-pack" => asset!("/assets/icons/favicon-pack.svg"),
        "rename-batch" => asset!("/assets/icons/rename-batch.svg"),
        "zip-files" => asset!("/assets/icons/zip-files.svg"),
        "unzip" => asset!("/assets/icons/unzip.svg"),
        "encrypt-file" => asset!("/assets/icons/encrypt-file.svg"),
        "decrypt-file" => asset!("/assets/icons/decrypt-file.svg"),
        "video-to-gif" => asset!("/assets/icons/video-to-gif.svg"),
        "trim-video" => asset!("/assets/icons/trim-video.svg"),
        "convert-video" => asset!("/assets/icons/convert-video.svg"),
        _ => return None,
    })
}
