//! PrivZapp core: shared types, the tool registry, and small parsing helpers.
//!
//! Every tool in PrivZapp runs entirely on the user's device. This crate holds
//! the data model shared by the engine crates and the UI; it has zero
//! dependencies so it compiles everywhere (native + wasm32) instantly.

#![forbid(unsafe_code)]

pub mod seo;

use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PzError {
    /// The user gave us something we can't work with (bad range, no files...).
    Invalid(String),
    /// Valid request, but we don't support it (yet).
    Unsupported(String),
    /// The operation itself failed (corrupt file, encoder error...).
    Failed(String),
}

impl fmt::Display for PzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PzError::Invalid(m) => write!(f, "Invalid input: {m}"),
            PzError::Unsupported(m) => write!(f, "Unsupported: {m}"),
            PzError::Failed(m) => write!(f, "Operation failed: {m}"),
        }
    }
}

impl std::error::Error for PzError {}

// ---------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------

/// A file loaded in memory. Bytes never leave the device.
#[derive(Debug, Clone)]
pub struct InputFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// A processed result, ready to hand back to the user.
#[derive(Debug, Clone)]
pub struct OutputFile {
    pub name: String,
    pub mime: &'static str,
    pub bytes: Vec<u8>,
}

/// Filename without its extension ("report.pdf" -> "report").
pub fn stem(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((s, _)) if !s.is_empty() => s,
        _ => name,
    }
}

/// Human-readable size ("3.4 MB").
pub fn human_size(bytes: usize) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

// ---------------------------------------------------------------------------
// Tool registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    Pdf,
    Image,
    Archive,
    Security,
}

impl ToolCategory {
    pub fn label(&self) -> &'static str {
        match self {
            ToolCategory::Pdf => "PDF",
            ToolCategory::Image => "Image",
            ToolCategory::Archive => "Compress",
            ToolCategory::Security => "Protect",
        }
    }
}

/// Which option widgets a tool's page should render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionKind {
    /// 1..=100 quality slider (lossy encoders).
    Quality,
    /// Width/height inputs; 0 means "keep aspect ratio from the other side".
    Dimensions,
    /// Output format select for image conversion.
    TargetFormat,
    /// Page range text input, e.g. "1-3,5". Empty = every page separately.
    PageRange,
    /// 90 / 180 / 270 degrees.
    RotateAngle,
    /// Watermark text input.
    WatermarkText,
    /// Ordered page list, e.g. "3,1,2". Order matters, duplicates allowed.
    PageOrder,
    /// X/Y offset + width/height inputs for cropping.
    CropRect,
    /// Password input (used on-device only, never transmitted).
    Password,
    /// Rename pattern, e.g. "vacation-{n}".
    NamePattern,
    /// Output resolution as a percentage of the original (10–100).
    ResolutionPercent,
    /// Left/top/right/bottom margin inputs (PDF points or pixels).
    Margins,
    /// Horizontal / vertical mirror select.
    FlipDir,
    /// 2x / 4x upscale factor select.
    ScaleFactor,
    /// 1..=100 effect-strength slider (e.g. blur).
    Strength,
}

#[derive(Debug, Clone, Copy)]
pub struct ToolMeta {
    pub slug: &'static str,
    pub name: &'static str,
    pub tagline: &'static str,
    pub category: ToolCategory,
    /// Value for the file input's `accept` attribute.
    pub accept: &'static str,
    /// Whether the tool takes multiple input files.
    pub multi: bool,
    /// Minimum number of files required to run.
    pub min_files: usize,
    pub options: &'static [OptionKind],
    pub icon: &'static str,
}

pub const TOOLS: &[ToolMeta] = &[
    ToolMeta {
        slug: "edit-pdf",
        name: "Edit PDF",
        tagline: "Sign, draw, stamp, rotate, watermark — one workspace",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: false,
        min_files: 1,
        // The editor page is bespoke; the generic options system isn't used.
        options: &[],
        icon: "✏️",
    },
    ToolMeta {
        slug: "merge-pdf",
        name: "Merge PDF",
        tagline: "Combine PDFs in the order you choose",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: true,
        min_files: 2,
        options: &[],
        icon: "🗂️",
    },
    ToolMeta {
        slug: "split-pdf",
        name: "Split PDF",
        tagline: "Extract pages or burst into single pages",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: false,
        min_files: 1,
        options: &[OptionKind::PageRange],
        icon: "✂️",
    },
    ToolMeta {
        slug: "rotate-pdf",
        name: "Rotate PDF",
        tagline: "Rotate every page by 90, 180 or 270 degrees",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: true,
        min_files: 1,
        options: &[OptionKind::RotateAngle],
        icon: "🔄",
    },
    ToolMeta {
        slug: "compress-pdf",
        name: "Compress PDF",
        tagline: "Shrink PDFs by recompressing their streams",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: true,
        min_files: 1,
        options: &[],
        icon: "🗜️",
    },
    ToolMeta {
        slug: "images-to-pdf",
        name: "Images to PDF",
        tagline: "Turn photos and scans into one PDF",
        category: ToolCategory::Pdf,
        accept: "image/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::Quality],
        icon: "🖼️",
    },
    ToolMeta {
        slug: "watermark-pdf",
        name: "Watermark PDF",
        tagline: "Stamp text across every page",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: true,
        min_files: 1,
        options: &[OptionKind::WatermarkText],
        icon: "💧",
    },
    ToolMeta {
        slug: "reorder-pdf",
        name: "Reorder PDF",
        tagline: "Rearrange, duplicate or drop pages",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: false,
        min_files: 1,
        options: &[OptionKind::PageOrder],
        icon: "🔀",
    },
    ToolMeta {
        slug: "page-numbers-pdf",
        name: "Add Page Numbers",
        tagline: "Stamp page numbers on every PDF page",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: true,
        min_files: 1,
        options: &[],
        icon: "🔢",
    },
    ToolMeta {
        slug: "crop-pdf",
        name: "Crop PDF",
        tagline: "Trim margins off every page",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: true,
        min_files: 1,
        options: &[OptionKind::Margins],
        icon: "✂️",
    },
    ToolMeta {
        slug: "extract-text-pdf",
        name: "PDF to Text",
        tagline: "Pull all text out of a PDF",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: true,
        min_files: 1,
        options: &[],
        icon: "📝",
    },
    ToolMeta {
        slug: "repair-pdf",
        name: "Repair PDF",
        tagline: "Rebuild a damaged PDF's structure",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: true,
        min_files: 1,
        options: &[],
        icon: "🩹",
    },
    ToolMeta {
        slug: "protect-pdf",
        name: "Protect PDF",
        tagline: "Password-protect with AES-256, opens anywhere",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: true,
        min_files: 1,
        options: &[OptionKind::Password],
        icon: "🛡️",
    },
    ToolMeta {
        slug: "unlock-pdf",
        name: "Unlock PDF",
        tagline: "Remove a password you know from a PDF",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: true,
        min_files: 1,
        options: &[OptionKind::Password],
        icon: "🔑",
    },
    ToolMeta {
        slug: "convert-img",
        name: "Convert Image",
        tagline: "PNG, JPG, WebP, GIF, BMP, TIFF, ICO — any to any",
        category: ToolCategory::Image,
        accept: "image/*",
        multi: true,
        min_files: 1,
        options: &[
            OptionKind::TargetFormat,
            OptionKind::Quality,
            OptionKind::ResolutionPercent,
        ],
        icon: "🔁",
    },
    ToolMeta {
        slug: "resize-img",
        name: "Resize Image",
        tagline: "Exact pixels or keep the aspect ratio",
        category: ToolCategory::Image,
        accept: "image/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::Dimensions],
        icon: "📐",
    },
    ToolMeta {
        slug: "compress-img",
        name: "Compress Image",
        tagline: "Smaller files, same format",
        category: ToolCategory::Image,
        accept: "image/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::Quality, OptionKind::ResolutionPercent],
        icon: "🪶",
    },
    ToolMeta {
        slug: "rotate-img",
        name: "Rotate Image",
        tagline: "Turn photos by 90, 180 or 270 degrees",
        category: ToolCategory::Image,
        accept: "image/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::RotateAngle],
        icon: "🔃",
    },
    ToolMeta {
        slug: "flip-img",
        name: "Flip Image",
        tagline: "Mirror horizontally or vertically",
        category: ToolCategory::Image,
        accept: "image/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::FlipDir],
        icon: "🪞",
    },
    ToolMeta {
        slug: "upscale-img",
        name: "Upscale Image",
        tagline: "Enlarge 2x or 4x with sharp resampling",
        category: ToolCategory::Image,
        accept: "image/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::ScaleFactor, OptionKind::Quality],
        icon: "🔍",
    },
    ToolMeta {
        slug: "grayscale-img",
        name: "Grayscale Image",
        tagline: "Convert photos to black and white",
        category: ToolCategory::Image,
        accept: "image/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::Quality],
        icon: "🌗",
    },
    ToolMeta {
        slug: "blur-img",
        name: "Blur Image",
        tagline: "Soften a picture with gaussian blur",
        category: ToolCategory::Image,
        accept: "image/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::Strength],
        icon: "🌫️",
    },
    ToolMeta {
        slug: "watermark-img",
        name: "Watermark Image",
        tagline: "Stamp text over your pictures",
        category: ToolCategory::Image,
        accept: "image/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::WatermarkText, OptionKind::Quality],
        icon: "💦",
    },
    ToolMeta {
        slug: "strip-exif",
        name: "Strip Metadata",
        tagline: "Remove EXIF location, camera and edit data",
        category: ToolCategory::Image,
        accept: "image/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::Quality],
        icon: "🧽",
    },
    ToolMeta {
        slug: "crop-img",
        name: "Crop Image",
        tagline: "Cut out exactly the pixels you want",
        category: ToolCategory::Image,
        accept: "image/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::CropRect],
        icon: "🔲",
    },
    ToolMeta {
        slug: "favicon-pack",
        name: "Favicon Generator",
        tagline: "Any image → complete favicon pack (.zip)",
        category: ToolCategory::Image,
        accept: "image/*",
        multi: false,
        min_files: 1,
        options: &[],
        icon: "🌐",
    },
    ToolMeta {
        slug: "rename-batch",
        name: "Batch Rename",
        tagline: "Rename many files with one pattern",
        category: ToolCategory::Image,
        accept: "*/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::NamePattern],
        icon: "🏷️",
    },
    ToolMeta {
        slug: "zip-files",
        name: "Create ZIP",
        tagline: "Bundle any files into one archive",
        category: ToolCategory::Archive,
        accept: "*/*",
        multi: true,
        min_files: 1,
        options: &[],
        icon: "📦",
    },
    ToolMeta {
        slug: "unzip",
        name: "Extract ZIP",
        tagline: "Unpack an archive, straight in your browser",
        category: ToolCategory::Archive,
        accept: ".zip",
        multi: false,
        min_files: 1,
        options: &[],
        icon: "📂",
    },
    ToolMeta {
        slug: "encrypt-file",
        name: "Encrypt File",
        tagline: "Lock any file with AES-256 and a password",
        category: ToolCategory::Security,
        accept: "*/*",
        multi: true,
        min_files: 1,
        options: &[OptionKind::Password],
        icon: "🔐",
    },
    ToolMeta {
        slug: "decrypt-file",
        name: "Decrypt File",
        tagline: "Unlock a .pzv vault with its password",
        category: ToolCategory::Security,
        accept: ".pzv",
        multi: true,
        min_files: 1,
        options: &[OptionKind::Password],
        icon: "🔓",
    },
];

pub fn tool_by_slug(slug: &str) -> Option<&'static ToolMeta> {
    TOOLS.iter().find(|t| t.slug == slug)
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Runtime options collected from the tool page. One struct for all tools;
/// each tool reads only the fields its `OptionKind`s cover.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolOptions {
    pub quality: u8,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub pages: String,
    pub angle: i32,
    /// Free text used by WatermarkText and NamePattern tools.
    pub text: String,
    /// Crop offset (used with `width`/`height` by CropRect tools).
    pub x: u32,
    pub y: u32,
    /// Password for encrypt/decrypt. Stays in memory on this device only.
    pub password: String,
    /// Upscale factor (2 or 4).
    pub scale: u32,
    /// Output resolution percentage for compressors (10–100; 100 = keep).
    pub percent: u32,
}

impl Default for ToolOptions {
    fn default() -> Self {
        Self {
            quality: 80,
            width: 0,
            height: 0,
            format: "png".to_string(),
            pages: String::new(),
            angle: 90,
            text: String::new(),
            x: 0,
            y: 0,
            password: String::new(),
            scale: 2,
            percent: 100,
        }
    }
}

/// Parse a 1-based page-range spec like "1-3,5,9-10" into a sorted,
/// de-duplicated page list, validated against `total` pages.
pub fn parse_page_ranges(spec: &str, total: u32) -> Result<Vec<u32>, PzError> {
    let mut pages = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (lo, hi) = match part.split_once('-') {
            Some((a, b)) => {
                let a: u32 = a.trim().parse().map_err(|_| bad_range(part))?;
                let b: u32 = b.trim().parse().map_err(|_| bad_range(part))?;
                (a, b)
            }
            None => {
                let p: u32 = part.parse().map_err(|_| bad_range(part))?;
                (p, p)
            }
        };
        if lo == 0 || hi < lo || hi > total {
            return Err(PzError::Invalid(format!(
                "page range \"{part}\" is out of bounds (document has {total} pages)"
            )));
        }
        pages.extend(lo..=hi);
    }
    pages.sort_unstable();
    pages.dedup();
    if pages.is_empty() {
        return Err(PzError::Invalid(
            "no pages selected — use e.g. \"1-3,5\"".to_string(),
        ));
    }
    Ok(pages)
}

fn bad_range(part: &str) -> PzError {
    PzError::Invalid(format!("could not parse page range \"{part}\""))
}

/// Parse a 1-based page-order spec like "3,1-2" into the exact page sequence
/// it describes. Unlike [`parse_page_ranges`], order is preserved and
/// duplicates are allowed (listing a page twice duplicates it).
pub fn parse_page_order(spec: &str, total: u32) -> Result<Vec<u32>, PzError> {
    let mut pages = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (lo, hi) = match part.split_once('-') {
            Some((a, b)) => {
                let a: u32 = a.trim().parse().map_err(|_| bad_range(part))?;
                let b: u32 = b.trim().parse().map_err(|_| bad_range(part))?;
                (a, b)
            }
            None => {
                let p: u32 = part.parse().map_err(|_| bad_range(part))?;
                (p, p)
            }
        };
        if lo == 0 || hi < lo || hi > total {
            return Err(PzError::Invalid(format!(
                "page \"{part}\" is out of bounds (document has {total} pages)"
            )));
        }
        pages.extend(lo..=hi);
    }
    if pages.is_empty() {
        return Err(PzError::Invalid(
            "no pages given — use e.g. \"3,1,2\"".to_string(),
        ));
    }
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ranges() {
        assert_eq!(parse_page_ranges("1-3,5", 10).unwrap(), vec![1, 2, 3, 5]);
        assert_eq!(parse_page_ranges("3, 1", 3).unwrap(), vec![1, 3]);
        assert!(parse_page_ranges("0", 3).is_err());
        assert!(parse_page_ranges("4", 3).is_err());
        assert!(parse_page_ranges("", 3).is_err());
        assert!(parse_page_ranges("2-1", 3).is_err());
    }

    #[test]
    fn parses_order() {
        assert_eq!(parse_page_order("3,1-2", 3).unwrap(), vec![3, 1, 2]);
        assert_eq!(parse_page_order("1,1", 2).unwrap(), vec![1, 1]);
        assert!(parse_page_order("4", 3).is_err());
        assert!(parse_page_order("", 3).is_err());
    }

    #[test]
    fn stems() {
        assert_eq!(stem("a.pdf"), "a");
        assert_eq!(stem("archive.tar.gz"), "archive.tar");
        assert_eq!(stem("noext"), "noext");
        assert_eq!(stem(".hidden"), ".hidden");
    }

    #[test]
    fn registry_slugs_unique() {
        let mut slugs: Vec<_> = TOOLS.iter().map(|t| t.slug).collect();
        slugs.sort_unstable();
        let len = slugs.len();
        slugs.dedup();
        assert_eq!(len, slugs.len());
    }
}
