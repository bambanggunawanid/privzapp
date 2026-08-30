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
    Video,
}

impl ToolCategory {
    pub fn label(&self) -> &'static str {
        match self {
            ToolCategory::Pdf => "PDF",
            ToolCategory::Image => "Image",
            ToolCategory::Archive => "Compress",
            ToolCategory::Security => "Protect",
            ToolCategory::Video => "Video",
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
    /// Output format select for PDF page rasterization (PNG/JPG/WebP).
    RasterFormat,
    /// Output container select for video conversion (MP4/WebM/MKV/MOV/AVI).
    VideoFormat,
    /// Output format select for audio extraction (MP3/WAV/OGG/M4A).
    AudioFormat,
    /// Output frame rate select for GIF conversion.
    Fps,
    /// Optional start/end timecodes ("90", "1:30" or "1:30:05.5").
    TimeRange,
    /// 1x-4x render scale for PDF page rasterization (1x = 72 DPI).
    RenderScale,
}

/// How a tool's work actually gets done.
///
/// Almost everything is `Engine`: a pure `pz_engine::run` call, bytes in →
/// bytes out, off on a Web Worker (ADR-0002/0004). `BrowserRender` marks
/// the exception — a tool that needs the browser to *rasterize* a PDF
/// first, which no pure-Rust crate can do (ADR-0009). The app dispatches
/// on this instead of matching slugs, so a new tool has to state which it
/// is, and `pz_engine::run` rejects `BrowserRender` slugs outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPipeline {
    /// Pure engine call. The default for every tool.
    Engine,
    /// Pages are rendered in the browser (PDF.js), then the engine packages
    /// the result. Web/desktop only — there is no headless path.
    BrowserRender,
    /// The bundled ffmpeg.wasm does the work in a Web Worker (ADR-0010).
    /// Same rule: browser only, and `pz_engine::run` refuses the slug.
    BrowserFfmpeg,
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
    /// Which execution path runs this tool (see `ToolPipeline`).
    pub pipeline: ToolPipeline,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
    },
    ToolMeta {
        slug: "pdf-to-images",
        name: "PDF to Image",
        tagline: "Every page as a PNG, JPG or WebP",
        category: ToolCategory::Pdf,
        accept: ".pdf",
        multi: false,
        min_files: 1,
        options: &[
            OptionKind::RasterFormat,
            OptionKind::RenderScale,
            OptionKind::Quality,
            OptionKind::PageRange,
        ],
        icon: "🖼️",
        // Rasterization has no pure-Rust path — the browser renders the
        // pages and the engine zips them (ADR-0009).
        pipeline: ToolPipeline::BrowserRender,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
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
        pipeline: ToolPipeline::Engine,
    },
    ToolMeta {
        slug: "video-to-gif",
        name: "Video to GIF",
        tagline: "Turn any clip into a looping GIF",
        category: ToolCategory::Video,
        accept: "video/*",
        multi: false,
        min_files: 1,
        options: &[
            OptionKind::Fps,
            OptionKind::Dimensions,
            OptionKind::TimeRange,
        ],
        icon: "🎞️",
        pipeline: ToolPipeline::BrowserFfmpeg,
    },
    ToolMeta {
        slug: "trim-video",
        name: "Trim Video",
        tagline: "Cut a clip without re-encoding it",
        category: ToolCategory::Video,
        accept: "video/*",
        multi: false,
        min_files: 1,
        options: &[OptionKind::TimeRange],
        icon: "✂️",
        pipeline: ToolPipeline::BrowserFfmpeg,
    },
    ToolMeta {
        slug: "convert-video",
        name: "Convert Video",
        tagline: "MP4, WebM, MKV, MOV, AVI — GIFs in too",
        category: ToolCategory::Video,
        // .gif on purpose: an animated GIF is a fine video INPUT.
        accept: "video/*,.gif",
        multi: false,
        min_files: 1,
        options: &[OptionKind::VideoFormat, OptionKind::Quality],
        icon: "🎬",
        pipeline: ToolPipeline::BrowserFfmpeg,
    },
    ToolMeta {
        slug: "extract-audio",
        name: "Extract Audio",
        tagline: "Pull the soundtrack out as MP3, WAV, OGG or M4A",
        category: ToolCategory::Video,
        accept: "video/*",
        multi: false,
        min_files: 1,
        options: &[
            OptionKind::AudioFormat,
            OptionKind::Quality,
            OptionKind::TimeRange,
        ],
        icon: "🎵",
        pipeline: ToolPipeline::BrowserFfmpeg,
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
    /// Output frame rate for GIF conversion.
    pub fps: u32,
    /// Optional clip start/end timecodes (empty = start/end of the video).
    pub trim_start: String,
    pub trim_end: String,
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
            fps: 12,
            trim_start: String::new(),
            trim_end: String::new(),
        }
    }
}

/// Parse a timecode like "90", "1:30", "01:02:03" or "1:30.5" into
/// seconds. Fractions are allowed on the last field only; fields after
/// the first must stay below 60.
pub fn parse_timecode(spec: &str) -> Result<f64, PzError> {
    let spec = spec.trim();
    let bad = || PzError::Invalid(format!("\"{spec}\" is not a time — use seconds or mm:ss"));
    let parts: Vec<&str> = spec.split(':').collect();
    if spec.is_empty() || parts.len() > 3 {
        return Err(bad());
    }
    let mut secs = 0.0f64;
    for (i, part) in parts.iter().enumerate() {
        let last = i == parts.len() - 1;
        let v: f64 = part.parse().map_err(|_| bad())?;
        let whole_and_small = v.fract() == 0.0 && (i == 0 || v < 60.0);
        if !v.is_finite() || v < 0.0 || (!last && !whole_and_small) || (last && i > 0 && v >= 60.0)
        {
            return Err(bad());
        }
        secs = secs * 60.0 + v;
    }
    Ok(secs)
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
    fn timecodes_parse() {
        assert_eq!(parse_timecode("90").unwrap(), 90.0);
        assert_eq!(parse_timecode("1:30").unwrap(), 90.0);
        assert_eq!(parse_timecode("01:02:03").unwrap(), 3723.0);
        assert_eq!(parse_timecode(" 1:30.5 ").unwrap(), 90.5);
        assert_eq!(parse_timecode("0").unwrap(), 0.0);
        for bad in ["", "1:2:3:4", "1:75", "abc", "-5", "1.5:00", "2:60"] {
            assert!(parse_timecode(bad).is_err(), "{bad:?} should be rejected");
        }
    }

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
