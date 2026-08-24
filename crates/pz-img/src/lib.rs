//! Image operations: convert, resize, compress. Pure Rust, wasm32-safe
//! (no threads, no SIMD assumptions), everything in memory.

#![forbid(unsafe_code)]

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder};
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat};
use pz_core::{stem, OutputFile, PzError};

/// Formats offered in the "convert to" dropdown.
pub const TARGET_FORMATS: &[&str] = &["png", "jpg", "webp", "gif", "bmp", "tiff", "ico", "qoi"];

fn fmt_for_ext(ext: &str) -> Option<ImageFormat> {
    match ext.to_ascii_lowercase().as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "webp" => Some(ImageFormat::WebP),
        "gif" => Some(ImageFormat::Gif),
        "bmp" => Some(ImageFormat::Bmp),
        "tiff" | "tif" => Some(ImageFormat::Tiff),
        "ico" => Some(ImageFormat::Ico),
        "qoi" => Some(ImageFormat::Qoi),
        _ => None,
    }
}

fn ext_mime(fmt: ImageFormat) -> (&'static str, &'static str) {
    match fmt {
        ImageFormat::Png => ("png", "image/png"),
        ImageFormat::Jpeg => ("jpg", "image/jpeg"),
        ImageFormat::WebP => ("webp", "image/webp"),
        ImageFormat::Gif => ("gif", "image/gif"),
        ImageFormat::Bmp => ("bmp", "image/bmp"),
        ImageFormat::Tiff => ("tiff", "image/tiff"),
        ImageFormat::Ico => ("ico", "image/x-icon"),
        ImageFormat::Qoi => ("qoi", "image/qoi"),
        _ => ("bin", "application/octet-stream"),
    }
}

fn decode(bytes: &[u8]) -> Result<(DynamicImage, ImageFormat), PzError> {
    let fmt = image::guess_format(bytes)
        .map_err(|_| PzError::Unsupported("could not detect image format".into()))?;
    let img = image::load_from_memory(bytes)
        .map_err(|e| PzError::Failed(format!("could not decode image: {e}")))?;
    Ok((img, fmt))
}

/// Encode `img` as `fmt`. `quality` only affects lossy encoders (JPEG).
/// Lossy PNG for the compress/convert tools ONLY (geometry ops like
/// flip/rotate must stay lossless): encodes both the lossless original
/// and a palette-quantized version, returning whichever is smaller —
/// quantization wins on graphics/photos, lossless wins on smooth
/// synthetic gradients.
fn encode_png_best(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, PzError> {
    let lossless = encode(img, ImageFormat::Png, 100)?;
    if quality >= 100 {
        return Ok(lossless);
    }
    let quantized = encode(
        &DynamicImage::ImageRgba8(quantize_rgba(&img.to_rgba8(), quality)),
        ImageFormat::Png,
        100,
    )?;
    Ok(if quantized.len() < lossless.len() {
        quantized
    } else {
        lossless
    })
}

/// Reduce an image to a quality-scaled palette (16–256 colors) so the
/// PNG deflate stage has something to bite on. Colors are memoized per
/// unique input pixel, so graphics/screenshots quantize in ~O(unique).
fn quantize_rgba(rgba: &image::RgbaImage, quality: u8) -> image::RgbaImage {
    use std::collections::HashMap;
    let colors = (usize::from(quality.max(1)) * 256 / 100).clamp(16, 256);
    let nq = color_quant::NeuQuant::new(10, colors, rgba.as_raw());
    let palette = nq.color_map_rgba();
    let mut memo: HashMap<[u8; 4], [u8; 4]> = HashMap::new();
    let mut out = rgba.clone();
    for px in out.pixels_mut() {
        let mapped = *memo.entry(px.0).or_insert_with(|| {
            let i = nq.index_of(&px.0) * 4;
            [palette[i], palette[i + 1], palette[i + 2], palette[i + 3]]
        });
        px.0 = mapped;
    }
    out
}

fn encode(img: &DynamicImage, fmt: ImageFormat, quality: u8) -> Result<Vec<u8>, PzError> {
    let quality = quality.clamp(1, 100);
    let mut buf = Cursor::new(Vec::new());
    let res = match fmt {
        ImageFormat::Jpeg => {
            let mut enc = JpegEncoder::new_with_quality(&mut buf, quality);
            enc.encode_image(&img.to_rgb8()).map(|_| ())
        }
        ImageFormat::Png => {
            let enc =
                PngEncoder::new_with_quality(&mut buf, CompressionType::Best, PngFilter::Adaptive);
            img.write_with_encoder(enc)
        }
        // ICO caps dimensions at 256px; shrink first so conversion "just works".
        ImageFormat::Ico => {
            let small = if img.width() > 256 || img.height() > 256 {
                img.resize(256, 256, FilterType::Lanczos3)
            } else {
                img.clone()
            };
            DynamicImage::ImageRgba8(small.to_rgba8()).write_to(&mut buf, fmt)
        }
        // WebP encoding in `image` is lossless and wants RGB(A)8.
        ImageFormat::WebP => DynamicImage::ImageRgba8(img.to_rgba8()).write_to(&mut buf, fmt),
        _ => img.write_to(&mut buf, fmt),
    };
    res.map_err(|e| PzError::Failed(format!("could not encode image: {e}")))?;
    Ok(buf.into_inner())
}

/// Convert to `target` format ("png", "jpg", ...).
pub fn convert(name: &str, bytes: &[u8], target: &str, quality: u8) -> Result<OutputFile, PzError> {
    let fmt = fmt_for_ext(target)
        .ok_or_else(|| PzError::Unsupported(format!("unknown target format \"{target}\"")))?;
    let (img, _) = decode(bytes)?;
    let out = match fmt {
        ImageFormat::Png => encode_png_best(&img, quality)?,
        _ => encode(&img, fmt, quality)?,
    };
    let (ext, mime) = ext_mime(fmt);
    Ok(OutputFile {
        name: format!("{}.{ext}", stem(name)),
        mime,
        bytes: out,
    })
}

/// Resize. If one of `width`/`height` is 0, the aspect ratio is preserved;
/// if both are set the image is stretched to exactly that size.
/// Keeps the original format.
pub fn resize(
    name: &str,
    bytes: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<OutputFile, PzError> {
    if width == 0 && height == 0 {
        return Err(PzError::Invalid("set a width and/or a height".into()));
    }
    let (img, fmt) = decode(bytes)?;
    let (w0, h0) = (img.width(), img.height());
    let resized = if width > 0 && height > 0 {
        img.resize_exact(width, height, FilterType::Lanczos3)
    } else if width > 0 {
        let h = ((height_ratio(w0, h0) * width as f64).round() as u32).max(1);
        img.resize_exact(width, h, FilterType::Lanczos3)
    } else {
        let w = ((width_ratio(w0, h0) * height as f64).round() as u32).max(1);
        img.resize_exact(w, height, FilterType::Lanczos3)
    };
    let out = encode(&resized, fmt, quality)?;
    let (ext, mime) = ext_mime(fmt);
    Ok(OutputFile {
        name: format!(
            "{}-{}x{}.{ext}",
            stem(name),
            resized.width(),
            resized.height()
        ),
        mime,
        bytes: out,
    })
}

/// Decode any supported image and re-encode it as baseline RGB JPEG.
/// Returns `(width, height, jpeg_bytes)` — the shape `pz-pdf::from_jpegs`
/// embeds directly as a DCTDecode stream.
pub fn to_jpeg(bytes: &[u8], quality: u8) -> Result<(u32, u32, Vec<u8>), PzError> {
    let (img, _) = decode(bytes)?;
    let out = encode(&img, ImageFormat::Jpeg, quality)?;
    Ok((img.width(), img.height(), out))
}

/// Strip metadata (EXIF GPS/camera data, XMP, comments) by decoding to raw
/// pixels and re-encoding. Encoders in `image` write no metadata, so nothing
/// survives the round trip. Lossy formats are re-encoded at `quality`.
pub fn strip_metadata(name: &str, bytes: &[u8], quality: u8) -> Result<OutputFile, PzError> {
    let (img, fmt) = decode(bytes)?;
    let out = encode(&img, fmt, quality)?;
    let (ext, mime) = ext_mime(fmt);
    Ok(OutputFile {
        name: format!("{}-clean.{ext}", stem(name)),
        mime,
        bytes: out,
    })
}

/// Crop to the `w`×`h` rectangle whose top-left corner is at (`x`, `y`).
/// Keeps the original format.
pub fn crop(
    name: &str,
    bytes: &[u8],
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    quality: u8,
) -> Result<OutputFile, PzError> {
    if w == 0 || h == 0 {
        return Err(PzError::Invalid("set a crop width and height".into()));
    }
    let (img, fmt) = decode(bytes)?;
    if x.saturating_add(w) > img.width() || y.saturating_add(h) > img.height() {
        return Err(PzError::Invalid(format!(
            "crop rectangle exceeds the image ({}x{})",
            img.width(),
            img.height()
        )));
    }
    let cropped = img.crop_imm(x, y, w, h);
    let out = encode(&cropped, fmt, quality)?;
    let (ext, mime) = ext_mime(fmt);
    Ok(OutputFile {
        name: format!("{}-crop.{ext}", stem(name)),
        mime,
        bytes: out,
    })
}

/// Rotate by 90/180/270 degrees clockwise. Keeps the original format.
pub fn rotate(name: &str, bytes: &[u8], angle: i32, quality: u8) -> Result<OutputFile, PzError> {
    let (img, fmt) = decode(bytes)?;
    let rotated = match angle {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => return Err(PzError::Invalid("angle must be 90, 180 or 270".into())),
    };
    let out = encode(&rotated, fmt, quality)?;
    let (ext, mime) = ext_mime(fmt);
    Ok(OutputFile {
        name: format!("{}-rotated.{ext}", stem(name)),
        mime,
        bytes: out,
    })
}

/// Mirror horizontally (default) or vertically. Keeps the original format.
pub fn flip(name: &str, bytes: &[u8], direction: &str, quality: u8) -> Result<OutputFile, PzError> {
    let (img, fmt) = decode(bytes)?;
    let flipped = if direction.eq_ignore_ascii_case("vertical") {
        img.flipv()
    } else {
        img.fliph()
    };
    let out = encode(&flipped, fmt, quality)?;
    let (ext, mime) = ext_mime(fmt);
    Ok(OutputFile {
        name: format!("{}-flipped.{ext}", stem(name)),
        mime,
        bytes: out,
    })
}

/// Convert to grayscale (kept in the original file format).
pub fn grayscale(name: &str, bytes: &[u8], quality: u8) -> Result<OutputFile, PzError> {
    let (img, fmt) = decode(bytes)?;
    // Keep an 8-bit luma+alpha buffer so formats with alpha stay intact.
    let gray = DynamicImage::ImageLumaA8(img.to_luma_alpha8());
    let out = encode(&gray, fmt, quality)?;
    let (ext, mime) = ext_mime(fmt);
    Ok(OutputFile {
        name: format!("{}-grayscale.{ext}", stem(name)),
        mime,
        bytes: out,
    })
}

/// Upscale by an integer factor (2x/4x) with Lanczos resampling.
pub fn upscale(name: &str, bytes: &[u8], factor: u32, quality: u8) -> Result<OutputFile, PzError> {
    if !matches!(factor, 2 | 4) {
        return Err(PzError::Invalid("upscale factor must be 2 or 4".into()));
    }
    let (img, fmt) = decode(bytes)?;
    let (w, h) = (img.width() * factor, img.height() * factor);
    const MAX_PIXELS: u64 = 64_000_000; // ~16k x 4k — keep wasm memory sane
    if w as u64 * h as u64 > MAX_PIXELS {
        return Err(PzError::Invalid(format!(
            "result would be {w}x{h} — too large to process in memory"
        )));
    }
    let up = img.resize_exact(w, h, FilterType::Lanczos3);
    let out = encode(&up, fmt, quality)?;
    let (ext, mime) = ext_mime(fmt);
    Ok(OutputFile {
        name: format!("{}-{factor}x.{ext}", stem(name)),
        mime,
        bytes: out,
    })
}

/// Gaussian blur; `strength` 1..=100 maps to sigma 0.5..=15.
pub fn blur(name: &str, bytes: &[u8], strength: u8, quality: u8) -> Result<OutputFile, PzError> {
    let strength = strength.clamp(1, 100);
    let sigma = 0.5 + (strength as f32 - 1.0) * (14.5 / 99.0);
    let (img, fmt) = decode(bytes)?;
    let blurred = img.blur(sigma);
    let out = encode(&blurred, fmt, quality)?;
    let (ext, mime) = ext_mime(fmt);
    Ok(OutputFile {
        name: format!("{}-blur.{ext}", stem(name)),
        mime,
        bytes: out,
    })
}

/// Stamp semi-transparent text across the middle of the image.
/// Text is rasterized from an embedded Liberation Sans (SIL OFL 1.1).
pub fn watermark_text(
    name: &str,
    bytes: &[u8],
    text: &str,
    quality: u8,
) -> Result<OutputFile, PzError> {
    use ab_glyph::{Font, FontRef, Glyph, ScaleFont};

    let text = text.trim();
    if text.is_empty() {
        return Err(PzError::Invalid("enter the watermark text".into()));
    }
    let (img, fmt) = decode(bytes)?;
    let mut rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());

    static FONT_BYTES: &[u8] = include_bytes!("../fonts/LiberationSans-Regular.ttf");
    let font = FontRef::try_from_slice(FONT_BYTES)
        .map_err(|_| PzError::Failed("embedded font failed to load".into()))?;

    // Size the text to ~80% of the image width.
    let probe = font.as_scaled(100.0);
    let probe_width: f32 = text
        .chars()
        .map(|c| probe.h_advance(probe.glyph_id(c)))
        .sum();
    if probe_width <= 0.0 {
        return Err(PzError::Invalid(
            "watermark text has no drawable glyphs".into(),
        ));
    }
    let scale = (w as f32 * 0.8 * 100.0 / probe_width).clamp(8.0, h as f32 * 0.5);
    let scaled = font.as_scaled(scale);

    let text_width: f32 = text
        .chars()
        .map(|c| scaled.h_advance(scaled.glyph_id(c)))
        .sum();
    let mut x = (w as f32 - text_width) / 2.0;
    let baseline = h as f32 / 2.0 + scaled.ascent() / 2.5;

    for c in text.chars() {
        let id = scaled.glyph_id(c);
        let glyph = Glyph {
            id,
            scale: scale.into(),
            position: ab_glyph::point(x, baseline),
        };
        x += scaled.h_advance(id);
        if let Some(outline) = font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            outline.draw(|gx, gy, cov| {
                let px = bounds.min.x as i64 + gx as i64;
                let py = bounds.min.y as i64 + gy as i64;
                if px < 0 || py < 0 || px >= w as i64 || py >= h as i64 {
                    return;
                }
                let p = rgba.get_pixel_mut(px as u32, py as u32);
                // White stamp at ~45% opacity, scaled by glyph coverage.
                let a = cov * 0.45;
                for ch in 0..3 {
                    p[ch] = (p[ch] as f32 * (1.0 - a) + 255.0 * a) as u8;
                }
            });
        }
    }

    let out = encode(&DynamicImage::ImageRgba8(rgba), fmt, quality)?;
    let (ext, mime) = ext_mime(fmt);
    Ok(OutputFile {
        name: format!("{}-watermarked.{ext}", stem(name)),
        mime,
        bytes: out,
    })
}

/// Sizes shipped in the favicon pack (PNG); 16/32/48 also go into the ICO.
const FAVICON_PNG_SIZES: &[(u32, &str)] = &[
    (16, "favicon-16x16.png"),
    (32, "favicon-32x32.png"),
    (180, "apple-touch-icon.png"),
    (192, "android-chrome-192x192.png"),
    (512, "android-chrome-512x512.png"),
];

/// Build the standard favicon file set from any image: a multi-size
/// `favicon.ico` (16/32/48) plus the PNG sizes modern browsers and mobile
/// platforms expect. Non-square inputs are center-cropped first.
pub fn favicon_images(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, PzError> {
    use image::codecs::ico::{IcoEncoder, IcoFrame};

    let (img, _) = decode(bytes)?;
    let side = img.width().min(img.height());
    if side < 16 {
        return Err(PzError::Invalid(
            "image is too small — needs at least 16x16 pixels".into(),
        ));
    }
    let square = img.crop_imm(
        (img.width() - side) / 2,
        (img.height() - side) / 2,
        side,
        side,
    );
    let png_at = |s: u32| -> Result<Vec<u8>, PzError> {
        let resized = square.resize_exact(s, s, FilterType::Lanczos3);
        encode(
            &DynamicImage::ImageRgba8(resized.to_rgba8()),
            ImageFormat::Png,
            100,
        )
    };

    let (p16, p32) = (png_at(16)?, png_at(32)?);
    let mut ico = Vec::new();
    // IcoFrame::as_png takes RAW pixels and PNG-compresses them itself.
    let raw = |s: u32| square.resize_exact(s, s, FilterType::Lanczos3).to_rgba8();
    let (r16, r32, r48) = (raw(16), raw(32), raw(48));
    let frame = |buf: &[u8], s: u32| {
        IcoFrame::as_png(buf, s, s, image::ExtendedColorType::Rgba8)
            .map_err(|e| PzError::Failed(format!("could not build ICO frame: {e}")))
    };
    let frames = [
        frame(r16.as_raw(), 16)?,
        frame(r32.as_raw(), 32)?,
        frame(r48.as_raw(), 48)?,
    ];
    IcoEncoder::new(Cursor::new(&mut ico))
        .encode_images(&frames)
        .map_err(|e| PzError::Failed(format!("could not encode favicon.ico: {e}")))?;

    let mut out = vec![("favicon.ico".to_string(), ico)];
    for (size, name) in FAVICON_PNG_SIZES {
        let png = match *size {
            16 => p16.clone(),
            32 => p32.clone(),
            s => png_at(s)?,
        };
        out.push(((*name).to_string(), png));
    }
    Ok(out)
}

fn height_ratio(w: u32, h: u32) -> f64 {
    h as f64 / w as f64
}

fn width_ratio(w: u32, h: u32) -> f64 {
    w as f64 / h as f64
}

/// Re-encode in the same format with stronger compression. For JPEG the
/// quality slider applies; PNG gets max lossless compression.
/// `percent` scales the output resolution (10–100; anything else means
/// keep) — the biggest size lever once quality has done its part.
pub fn compress(
    name: &str,
    bytes: &[u8],
    quality: u8,
    percent: u32,
) -> Result<OutputFile, PzError> {
    let (img, fmt) = decode(bytes)?;
    let img = if (10..100).contains(&percent) {
        let w = (img.width() * percent / 100).max(1);
        let h = (img.height() * percent / 100).max(1);
        img.resize_exact(w, h, FilterType::Lanczos3)
    } else {
        img
    };
    let out = match fmt {
        ImageFormat::Png => encode_png_best(&img, quality)?,
        _ => encode(&img, fmt, quality)?,
    };
    let (ext, mime) = ext_mime(fmt);
    // Never hand back a bigger file than the original.
    let (final_bytes, note) = if out.len() < bytes.len() {
        (out, "-compressed")
    } else {
        (bytes.to_vec(), "")
    };
    Ok(OutputFile {
        name: format!("{}{note}.{ext}", stem(name)),
        mime,
        bytes: final_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_png() -> Vec<u8> {
        let img = DynamicImage::ImageRgba8(image::RgbaImage::from_fn(64, 48, |x, y| {
            image::Rgba([(x * 4) as u8, (y * 5) as u8, 128, 255])
        }));
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    #[test]
    fn convert_png_to_jpg() {
        let out = convert("photo.png", &sample_png(), "jpg", 80).unwrap();
        assert_eq!(out.name, "photo.jpg");
        assert_eq!(out.mime, "image/jpeg");
        assert_eq!(image::guess_format(&out.bytes).unwrap(), ImageFormat::Jpeg);
    }

    #[test]
    fn resize_keeps_aspect() {
        let out = resize("photo.png", &sample_png(), 32, 0, 80).unwrap();
        let img = image::load_from_memory(&out.bytes).unwrap();
        assert_eq!((img.width(), img.height()), (32, 24));
        assert_eq!(out.name, "photo-32x24.png");
    }

    #[test]
    fn resize_exact_both() {
        let out = resize("photo.png", &sample_png(), 10, 10, 80).unwrap();
        let img = image::load_from_memory(&out.bytes).unwrap();
        assert_eq!((img.width(), img.height()), (10, 10));
    }

    #[test]
    fn compress_never_grows() {
        let src = sample_png();
        let out = compress("photo.png", &src, 80, 100).unwrap();
        assert!(out.bytes.len() <= src.len());
    }

    #[test]
    fn compress_png_quality_actually_shrinks() {
        // Pseudo-noise: lossless PNG stays big, a 20%-quality palette
        // compresses far better — quality must actually shrink the file.
        let img = DynamicImage::ImageRgba8(image::RgbaImage::from_fn(128, 128, |x, y| {
            // Avalanche mix → true-ish noise (incompressible losslessly).
            let mut h = x.wrapping_mul(2654435761) ^ y.wrapping_mul(2246822519);
            h ^= h >> 15;
            h = h.wrapping_mul(2246822519);
            h ^= h >> 13;
            image::Rgba([h as u8, (h >> 8) as u8, (h >> 16) as u8, 255])
        }));
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Png).unwrap();
        let src = buf.into_inner();
        let hi = compress("c.png", &src, 100, 100).unwrap();
        let lo = compress("c.png", &src, 20, 100).unwrap();
        assert!(
            lo.bytes.len() < hi.bytes.len(),
            "q20 ({}) should be smaller than q100 ({})",
            lo.bytes.len(),
            hi.bytes.len()
        );
    }

    #[test]
    fn compress_resolution_percent_shrinks_dimensions_and_size() {
        let src = sample_png();
        let half = compress("photo.png", &src, 80, 50).unwrap();
        let img = image::load_from_memory(&half.bytes).unwrap();
        assert_eq!((img.width(), img.height()), (32, 24)); // 64x48 → 50%

        // Size must drop on content that doesn't compress losslessly.
        let noisy = DynamicImage::ImageRgba8(image::RgbaImage::from_fn(128, 128, |x, y| {
            let mut h = x.wrapping_mul(2654435761) ^ y.wrapping_mul(2246822519);
            h ^= h >> 15;
            h = h.wrapping_mul(2246822519);
            h ^= h >> 13;
            image::Rgba([h as u8, (h >> 8) as u8, (h >> 16) as u8, 255])
        }));
        let mut buf = Cursor::new(Vec::new());
        noisy.write_to(&mut buf, ImageFormat::Png).unwrap();
        let nsrc = buf.into_inner();
        let nfull = compress("n.png", &nsrc, 100, 100).unwrap();
        let nhalf = compress("n.png", &nsrc, 100, 50).unwrap();
        assert!(nhalf.bytes.len() < nfull.bytes.len());
    }

    #[test]
    fn convert_rejects_unknown_target() {
        assert!(convert("a.png", &sample_png(), "xyz", 80).is_err());
    }

    #[test]
    fn to_jpeg_reports_dimensions() {
        let (w, h, jpeg) = to_jpeg(&sample_png(), 80).unwrap();
        assert_eq!((w, h), (64, 48));
        assert_eq!(image::guess_format(&jpeg).unwrap(), ImageFormat::Jpeg);
    }

    /// A JPEG with a hand-spliced EXIF APP1 segment right after SOI.
    fn jpeg_with_exif() -> Vec<u8> {
        let (_, _, jpeg) = to_jpeg(&sample_png(), 90).unwrap();
        let payload = b"Exif\0\0FAKE-GPS-DATA";
        let mut out = Vec::with_capacity(jpeg.len() + payload.len() + 4);
        out.extend_from_slice(&jpeg[..2]); // SOI
        out.extend_from_slice(&[0xFF, 0xE1]);
        out.extend_from_slice(&((payload.len() as u16 + 2).to_be_bytes()));
        out.extend_from_slice(payload);
        out.extend_from_slice(&jpeg[2..]);
        out
    }

    #[test]
    fn strip_removes_exif() {
        let src = jpeg_with_exif();
        assert!(src.windows(4).any(|w| w == b"Exif"));
        let out = strip_metadata("photo.jpg", &src, 90).unwrap();
        assert!(!out.bytes.windows(4).any(|w| w == b"Exif"));
        assert_eq!(out.name, "photo-clean.jpg");
    }

    #[test]
    fn crops_rect() {
        let out = crop("photo.png", &sample_png(), 10, 8, 20, 16, 80).unwrap();
        let img = image::load_from_memory(&out.bytes).unwrap();
        assert_eq!((img.width(), img.height()), (20, 16));
        assert_eq!(out.name, "photo-crop.png");
    }

    #[test]
    fn crop_rejects_out_of_bounds() {
        assert!(crop("a.png", &sample_png(), 60, 0, 20, 10, 80).is_err());
        assert!(crop("a.png", &sample_png(), 0, 0, 0, 10, 80).is_err());
    }

    #[test]
    fn rotates_image() {
        let out = rotate("p.png", &sample_png(), 90, 80).unwrap();
        let img = image::load_from_memory(&out.bytes).unwrap();
        assert_eq!((img.width(), img.height()), (48, 64)); // 64x48 turned
        assert!(rotate("p.png", &sample_png(), 45, 80).is_err());
    }

    #[test]
    fn flips_image() {
        let src = image::load_from_memory(&sample_png()).unwrap().to_rgba8();
        let out = flip("p.png", &sample_png(), "horizontal", 80).unwrap();
        let img = image::load_from_memory(&out.bytes).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(0, 0), src.get_pixel(63, 0));
    }

    #[test]
    fn grayscales_image() {
        let out = grayscale("p.png", &sample_png(), 80).unwrap();
        let img = image::load_from_memory(&out.bytes).unwrap().to_rgba8();
        let p = img.get_pixel(10, 10);
        assert_eq!(p[0], p[1]);
        assert_eq!(p[1], p[2]);
    }

    #[test]
    fn upscales_2x() {
        let out = upscale("p.png", &sample_png(), 2, 80).unwrap();
        let img = image::load_from_memory(&out.bytes).unwrap();
        assert_eq!((img.width(), img.height()), (128, 96));
        assert!(upscale("p.png", &sample_png(), 3, 80).is_err());
    }

    #[test]
    fn blurs_image() {
        let out = blur("p.png", &sample_png(), 50, 80).unwrap();
        assert!(image::load_from_memory(&out.bytes).is_ok());
    }

    #[test]
    fn watermarks_image() {
        let src = sample_png();
        let out = watermark_text("p.png", &src, "DRAFT", 80).unwrap();
        assert_eq!(out.name, "p-watermarked.png");
        // The stamp must actually change pixels.
        let before = image::load_from_memory(&src).unwrap().to_rgba8();
        let after = image::load_from_memory(&out.bytes).unwrap().to_rgba8();
        assert!(before.pixels().zip(after.pixels()).any(|(a, b)| a != b));
        assert!(watermark_text("p.png", &src, "  ", 80).is_err());
    }
}
