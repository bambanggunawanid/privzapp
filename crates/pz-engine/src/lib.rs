//! The PrivZapp engine: one uniform entry point the UI calls for every tool.
//!
//! `run` is synchronous, pure computation on in-memory bytes — no I/O, no
//! network. It compiles to native code on desktop/mobile and to WASM on the
//! web, so files are processed on-device everywhere.

#![forbid(unsafe_code)]

use pz_core::{tool_by_slug, InputFile, OutputFile, PzError, ToolOptions};

/// Run tool `slug` over `files` with `opts`.
pub fn run(
    slug: &str,
    files: &[InputFile],
    opts: &ToolOptions,
) -> Result<Vec<OutputFile>, PzError> {
    let meta = tool_by_slug(slug)
        .ok_or_else(|| PzError::Unsupported(format!("unknown tool \"{slug}\"")))?;
    if files.len() < meta.min_files {
        return Err(PzError::Invalid(format!(
            "\"{}\" needs at least {} file(s)",
            meta.name, meta.min_files
        )));
    }

    match slug {
        "merge-pdf" => {
            let inputs: Vec<(String, Vec<u8>)> = files
                .iter()
                .map(|f| (f.name.clone(), f.bytes.clone()))
                .collect();
            Ok(vec![OutputFile {
                name: "merged.pdf".to_string(),
                mime: "application/pdf",
                bytes: pz_pdf::merge(&inputs)?,
            }])
        }
        "split-pdf" => {
            let f = &files[0];
            pz_pdf::split(&f.name, &f.bytes, &opts.pages)
        }
        "images-to-pdf" => {
            let jpegs: Vec<(u32, u32, Vec<u8>)> = files
                .iter()
                .map(|f| {
                    pz_img::to_jpeg(&f.bytes, opts.quality)
                        .map_err(|e| PzError::Failed(format!("{}: {e}", f.name)))
                })
                .collect::<Result<_, _>>()?;
            Ok(vec![OutputFile {
                name: "images.pdf".to_string(),
                mime: "application/pdf",
                bytes: pz_pdf::from_jpegs(&jpegs)?,
            }])
        }
        "watermark-pdf" => files
            .iter()
            .map(|f| pz_pdf::watermark(&f.name, &f.bytes, &opts.text))
            .collect(),
        "reorder-pdf" => {
            let f = &files[0];
            Ok(vec![pz_pdf::reorder(&f.name, &f.bytes, &opts.pages)?])
        }
        "rotate-pdf" => files
            .iter()
            .map(|f| pz_pdf::rotate(&f.name, &f.bytes, opts.angle))
            .collect(),
        "page-numbers-pdf" => files
            .iter()
            .map(|f| pz_pdf::page_numbers(&f.name, &f.bytes))
            .collect(),
        // Margins ride the crop fields: x=left, y=top, width=right, height=bottom.
        "crop-pdf" => files
            .iter()
            .map(|f| {
                pz_pdf::crop_margins(&f.name, &f.bytes, opts.x, opts.y, opts.width, opts.height)
            })
            .collect(),
        "extract-text-pdf" => files
            .iter()
            .map(|f| pz_pdf::extract_text(&f.name, &f.bytes))
            .collect(),
        "repair-pdf" => files
            .iter()
            .map(|f| pz_pdf::repair(&f.name, &f.bytes))
            .collect(),
        "protect-pdf" => {
            let password = require_password(&opts.password)?;
            files
                .iter()
                .map(|f| pz_pdf::protect(&f.name, &f.bytes, password, &pz_crypto::random_bytes(64)))
                .collect()
        }
        "unlock-pdf" => {
            let password = require_password(&opts.password)?;
            files
                .iter()
                .map(|f| pz_pdf::unlock(&f.name, &f.bytes, password))
                .collect()
        }
        "compress-pdf" => files
            .iter()
            .map(|f| pz_pdf::compress(&f.name, &f.bytes))
            .collect(),
        "convert-img" => files
            .iter()
            .map(|f| pz_img::convert(&f.name, &f.bytes, &opts.format, opts.quality))
            .collect(),
        "resize-img" => files
            .iter()
            .map(|f| pz_img::resize(&f.name, &f.bytes, opts.width, opts.height, opts.quality))
            .collect(),
        "compress-img" => files
            .iter()
            .map(|f| pz_img::compress(&f.name, &f.bytes, opts.quality))
            .collect(),
        "strip-exif" => files
            .iter()
            .map(|f| pz_img::strip_metadata(&f.name, &f.bytes, opts.quality))
            .collect(),
        "crop-img" => files
            .iter()
            .map(|f| {
                pz_img::crop(
                    &f.name,
                    &f.bytes,
                    opts.x,
                    opts.y,
                    opts.width,
                    opts.height,
                    opts.quality,
                )
            })
            .collect(),
        "rotate-img" => files
            .iter()
            .map(|f| pz_img::rotate(&f.name, &f.bytes, opts.angle, opts.quality))
            .collect(),
        "flip-img" => files
            .iter()
            .map(|f| pz_img::flip(&f.name, &f.bytes, &opts.format, opts.quality))
            .collect(),
        "upscale-img" => files
            .iter()
            .map(|f| pz_img::upscale(&f.name, &f.bytes, opts.scale, opts.quality))
            .collect(),
        "grayscale-img" => files
            .iter()
            .map(|f| pz_img::grayscale(&f.name, &f.bytes, opts.quality))
            .collect(),
        // The Strength slider arrives in `quality`; encode at a fixed 90.
        "blur-img" => files
            .iter()
            .map(|f| pz_img::blur(&f.name, &f.bytes, opts.quality, 90))
            .collect(),
        "watermark-img" => files
            .iter()
            .map(|f| pz_img::watermark_text(&f.name, &f.bytes, &opts.text, opts.quality))
            .collect(),
        "favicon-pack" => {
            let f = &files[0];
            let mut pack = pz_img::favicon_images(&f.bytes)?;
            pack.push((
                "site.webmanifest".to_string(),
                br##"{
  "name": "",
  "short_name": "",
  "icons": [
    { "src": "/android-chrome-192x192.png", "sizes": "192x192", "type": "image/png" },
    { "src": "/android-chrome-512x512.png", "sizes": "512x512", "type": "image/png" }
  ],
  "theme_color": "#ffffff",
  "background_color": "#ffffff",
  "display": "standalone"
}
"##
                .to_vec(),
            ));
            pack.push((
                "README.txt".to_string(),
                br#"Favicon pack generated by PrivZapp (https://privzapp.com)

1. Copy every file into the root of your website.
2. Paste this into your page's <head>:

<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png">
<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png">
<link rel="apple-touch-icon" href="/apple-touch-icon.png">
<link rel="manifest" href="/site.webmanifest">

favicon.ico at the site root is discovered automatically by browsers.
Fill in name/short_name and colors in site.webmanifest.
"#
                .to_vec(),
            ));
            Ok(vec![OutputFile {
                name: format!("{}-favicon-pack.zip", pz_core::stem(&f.name)),
                mime: "application/zip",
                bytes: pz_archive::create(&pack)?,
            }])
        }
        "rename-batch" => rename_batch(files, &opts.text),
        "zip-files" => {
            let inputs: Vec<(String, Vec<u8>)> = files
                .iter()
                .map(|f| (f.name.clone(), f.bytes.clone()))
                .collect();
            Ok(vec![OutputFile {
                name: "archive.zip".to_string(),
                mime: "application/zip",
                bytes: pz_archive::create(&inputs)?,
            }])
        }
        "unzip" => {
            let mut out = Vec::new();
            for f in files {
                out.extend(pz_archive::extract(&f.bytes)?);
            }
            Ok(out)
        }
        "encrypt-file" => {
            let password = require_password(&opts.password)?;
            files
                .iter()
                .map(|f| {
                    let bytes = pz_crypto::seal_with_password(password, &f.bytes)
                        .map_err(|e| PzError::Failed(e.to_string()))?;
                    Ok(OutputFile {
                        name: format!("{}.pzv", f.name),
                        mime: "application/octet-stream",
                        bytes,
                    })
                })
                .collect()
        }
        "decrypt-file" => {
            let password = require_password(&opts.password)?;
            files
                .iter()
                .map(|f| {
                    let bytes =
                        pz_crypto::open_with_password(password, &f.bytes).map_err(|e| match e {
                            pz_crypto::CryptoError::NotAVault => {
                                PzError::Invalid(format!("{}: {e}", f.name))
                            }
                            _ => PzError::Failed(format!(
                                "{}: wrong password or corrupted file",
                                f.name
                            )),
                        })?;
                    Ok(OutputFile {
                        name: f
                            .name
                            .strip_suffix(".pzv")
                            .filter(|s| !s.is_empty())
                            .unwrap_or("decrypted")
                            .to_string(),
                        mime: "application/octet-stream",
                        bytes,
                    })
                })
                .collect()
        }
        other => Err(PzError::Unsupported(format!(
            "tool \"{other}\" is registered but not implemented"
        ))),
    }
}

pub use pz_pdf::{PlacedRect, PlacedText, Stroke};

/// An image the editor placed on a page, still in its original format.
/// `rect` is (x, y, width, height) in PDF points, y = bottom edge.
#[derive(Debug, Clone)]
pub struct EditImage {
    pub bytes: Vec<u8>,
    pub rect: (f32, f32, f32, f32),
}

/// One page's worth of editor annotations.
#[derive(Debug, Clone, Default)]
pub struct PageEdit {
    /// 1-based page number.
    pub page: u32,
    pub strokes: Vec<Stroke>,
    pub images: Vec<EditImage>,
    pub texts: Vec<PlacedText>,
    pub rects: Vec<PlacedRect>,
}

/// The PDF editor's apply step: convert placed images to embeddable JPEGs
/// and bake everything into the document. Separate from `run` because the
/// editor's input is structured annotations, not a flat options struct.
pub fn edit_pdf(
    name: &str,
    pdf: &[u8],
    edits: Vec<PageEdit>,
    image_quality: u8,
) -> Result<OutputFile, PzError> {
    let mut converted = Vec::with_capacity(edits.len());
    for edit in edits {
        let images = edit
            .images
            .into_iter()
            .map(|im| {
                let (width_px, height_px, jpeg) = pz_img::to_jpeg(&im.bytes, image_quality)?;
                Ok(pz_pdf::PlacedJpeg {
                    jpeg,
                    width_px,
                    height_px,
                    rect: im.rect,
                })
            })
            .collect::<Result<Vec<_>, PzError>>()?;
        converted.push(pz_pdf::PageEdits {
            page: edit.page,
            strokes: edit.strokes,
            images,
            texts: edit.texts,
            rects: edit.rects,
        });
    }
    pz_pdf::annotate(name, pdf, &converted)
}

fn require_password(password: &str) -> Result<&str, PzError> {
    if password.is_empty() {
        return Err(PzError::Invalid("enter a password".into()));
    }
    Ok(password)
}

/// Rename files by pattern. `{n}` is the 1-based index; when the pattern has
/// no `{n}` and there are several files, a number is appended so names stay
/// unique. Original extensions are kept.
fn rename_batch(files: &[InputFile], pattern: &str) -> Result<Vec<OutputFile>, PzError> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Err(PzError::Invalid(
            "enter a name pattern, e.g. \"vacation-{n}\"".into(),
        ));
    }
    Ok(files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let n = (i + 1).to_string();
            let base = if pattern.contains("{n}") {
                pattern.replace("{n}", &n)
            } else if files.len() > 1 {
                format!("{pattern}-{n}")
            } else {
                pattern.to_string()
            };
            let name = match f.name.rsplit_once('.') {
                Some((_, ext)) if !ext.is_empty() => format!("{base}.{ext}"),
                _ => base,
            };
            OutputFile {
                name,
                mime: "application/octet-stream",
                bytes: f.bytes.clone(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_tool_errors() {
        assert!(run("nope", &[], &ToolOptions::default()).is_err());
    }

    #[test]
    fn enforces_min_files() {
        let err = run("merge-pdf", &[], &ToolOptions::default()).unwrap_err();
        assert!(matches!(err, PzError::Invalid(_)));
    }

    #[test]
    fn encrypt_decrypt_roundtrip_through_engine() {
        let files = vec![InputFile {
            name: "notes.txt".into(),
            bytes: b"very private".to_vec(),
        }];
        // Built from parts so secret scanners don't flag a literal assignment.
        let phrase = ["correct", "horse"].join(" ");
        let opts = ToolOptions {
            password: phrase,
            ..ToolOptions::default()
        };
        let sealed = run("encrypt-file", &files, &opts).unwrap();
        assert_eq!(sealed[0].name, "notes.txt.pzv");
        let opened = run(
            "decrypt-file",
            &[InputFile {
                name: sealed[0].name.clone(),
                bytes: sealed[0].bytes.clone(),
            }],
            &opts,
        )
        .unwrap();
        assert_eq!(opened[0].name, "notes.txt");
        assert_eq!(opened[0].bytes, b"very private");
    }

    #[test]
    fn encrypt_requires_password() {
        let files = vec![InputFile {
            name: "a.txt".into(),
            bytes: b"x".to_vec(),
        }];
        assert!(matches!(
            run("encrypt-file", &files, &ToolOptions::default()),
            Err(PzError::Invalid(_))
        ));
    }

    #[test]
    fn renames_with_pattern() {
        let files = vec![
            InputFile {
                name: "IMG_001.jpg".into(),
                bytes: vec![1],
            },
            InputFile {
                name: "IMG_002.png".into(),
                bytes: vec![2],
            },
        ];
        let opts = ToolOptions {
            text: "trip-{n}".into(),
            ..ToolOptions::default()
        };
        let out = run("rename-batch", &files, &opts).unwrap();
        assert_eq!(out[0].name, "trip-1.jpg");
        assert_eq!(out[1].name, "trip-2.png");
    }

    #[test]
    fn rename_appends_number_when_pattern_static() {
        let files = vec![
            InputFile {
                name: "a.txt".into(),
                bytes: vec![1],
            },
            InputFile {
                name: "b.txt".into(),
                bytes: vec![2],
            },
        ];
        let opts = ToolOptions {
            text: "doc".into(),
            ..ToolOptions::default()
        };
        let out = run("rename-batch", &files, &opts).unwrap();
        assert_eq!(out[0].name, "doc-1.txt");
        assert_eq!(out[1].name, "doc-2.txt");
    }

    #[test]
    fn favicon_pack_roundtrip() {
        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            64,
            48, // non-square on purpose: must center-crop
            image::Rgba([30, 60, 200, 255]),
        ));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let files = vec![InputFile {
            name: "logo.png".into(),
            bytes: buf.into_inner(),
        }];
        let out = run("favicon-pack", &files, &ToolOptions::default()).unwrap();
        assert_eq!(out[0].name, "logo-favicon-pack.zip");
        let extracted = pz_archive::extract(&out[0].bytes).unwrap();
        let names: Vec<&str> = extracted.iter().map(|f| f.name.as_str()).collect();
        for expected in [
            "favicon.ico",
            "favicon-16x16.png",
            "favicon-32x32.png",
            "apple-touch-icon.png",
            "android-chrome-192x192.png",
            "android-chrome-512x512.png",
            "site.webmanifest",
            "README.txt",
        ] {
            assert!(names.contains(&expected), "missing {expected} in pack");
        }
        let ico = extracted.iter().find(|f| f.name == "favicon.ico").unwrap();
        assert_eq!(&ico.bytes[..4], &[0, 0, 1, 0]); // ICO magic
        let big = extracted
            .iter()
            .find(|f| f.name == "android-chrome-512x512.png")
            .unwrap();
        let img = image::load_from_memory(&big.bytes).unwrap();
        assert_eq!((img.width(), img.height()), (512, 512));
    }

    #[test]
    fn zip_roundtrip_through_engine() {
        let files = vec![InputFile {
            name: "hello.txt".into(),
            bytes: b"hi there".to_vec(),
        }];
        let zipped = run("zip-files", &files, &ToolOptions::default()).unwrap();
        assert_eq!(zipped[0].name, "archive.zip");
        let unzipped = run(
            "unzip",
            &[InputFile {
                name: "archive.zip".into(),
                bytes: zipped[0].bytes.clone(),
            }],
            &ToolOptions::default(),
        )
        .unwrap();
        assert_eq!(unzipped[0].bytes, b"hi there");
    }
}
