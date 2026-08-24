//! PDF operations: merge, split, rotate, compress, watermark, reorder,
//! images→PDF. Built on `lopdf` (pure Rust), runs identically on native
//! targets and wasm32.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};
use pz_core::{parse_page_order, parse_page_ranges, stem, OutputFile, PzError};

const PDF_MIME: &str = "application/pdf";

fn load(bytes: &[u8]) -> Result<Document, PzError> {
    Document::load_mem(bytes).map_err(|e| PzError::Failed(format!("could not read PDF: {e}")))
}

fn save(mut doc: Document) -> Result<Vec<u8>, PzError> {
    let mut buf = Vec::new();
    doc.save_to(&mut buf)
        .map_err(|e| PzError::Failed(format!("could not write PDF: {e}")))?;
    Ok(buf)
}

/// Merge documents in the given order into one PDF.
///
/// This is the standard lopdf merge: renumber all objects into one id space,
/// re-parent every page under a single Pages node, and rebuild the catalog.
pub fn merge(inputs: &[(String, Vec<u8>)]) -> Result<Vec<u8>, PzError> {
    if inputs.len() < 2 {
        return Err(PzError::Invalid("select at least two PDFs to merge".into()));
    }

    let mut max_id = 1;
    let mut documents_pages: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut documents_objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut merged = Document::with_version("1.5");

    for (name, bytes) in inputs {
        let mut doc = load(bytes).map_err(|e| PzError::Failed(format!("{name}: {e}")))?;
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;
        for (_, object_id) in doc.get_pages() {
            let page = doc
                .get_object(object_id)
                .map_err(|e| PzError::Failed(format!("{name}: broken page tree: {e}")))?
                .to_owned();
            documents_pages.insert(object_id, page);
        }
        documents_objects.extend(doc.objects);
    }

    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    for (object_id, object) in documents_objects.iter() {
        match object.type_name().unwrap_or_default() {
            b"Catalog" => {
                catalog_object = Some((
                    catalog_object
                        .as_ref()
                        .map(|(id, _)| *id)
                        .unwrap_or(*object_id),
                    object.clone(),
                ));
            }
            b"Pages" => {
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref existing)) = pages_object {
                        if let Ok(old) = existing.as_dict() {
                            dictionary.extend(&old.clone());
                        }
                    }
                    pages_object = Some((
                        pages_object
                            .as_ref()
                            .map(|(id, _)| *id)
                            .unwrap_or(*object_id),
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            // Pages are re-inserted below; outlines/outline items are dropped.
            b"Page" | b"Outlines" | b"Outline" => {}
            _ => {
                merged.objects.insert(*object_id, object.clone());
            }
        }
    }

    let pages_object =
        pages_object.ok_or_else(|| PzError::Failed("no page tree found in inputs".into()))?;
    let catalog_object =
        catalog_object.ok_or_else(|| PzError::Failed("no catalog found in inputs".into()))?;

    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_object.0);
            merged
                .objects
                .insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    if let Ok(dictionary) = pages_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Count", documents_pages.len() as u32);
        dictionary.set(
            "Kids",
            documents_pages
                .keys()
                .map(|id| Object::Reference(*id))
                .collect::<Vec<_>>(),
        );
        merged
            .objects
            .insert(pages_object.0, Object::Dictionary(dictionary));
    }

    if let Ok(dictionary) = catalog_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_object.0);
        dictionary.remove(b"Outlines");
        merged
            .objects
            .insert(catalog_object.0, Object::Dictionary(dictionary));
    }

    merged.trailer.set("Root", catalog_object.0);
    merged.max_id = merged.objects.len() as u32;
    merged.renumber_objects();
    merged.compress();
    save(merged)
}

/// Split a PDF.
///
/// - Empty `pages_spec`: burst into one PDF per page.
/// - Otherwise: one PDF containing exactly the pages in the spec ("1-3,5").
pub fn split(name: &str, bytes: &[u8], pages_spec: &str) -> Result<Vec<OutputFile>, PzError> {
    let total = load(bytes)?.get_pages().len() as u32;
    if total == 0 {
        return Err(PzError::Failed("document has no pages".into()));
    }

    let selections: Vec<(String, Vec<u32>)> = if pages_spec.trim().is_empty() {
        (1..=total)
            .map(|p| (format!("{}-page-{p}.pdf", stem(name)), vec![p]))
            .collect()
    } else {
        let pages = parse_page_ranges(pages_spec, total)?;
        vec![(format!("{}-pages.pdf", stem(name)), pages)]
    };

    let mut outputs = Vec::with_capacity(selections.len());
    for (out_name, keep) in selections {
        let mut doc = load(bytes)?;
        let delete: Vec<u32> = (1..=total).filter(|p| !keep.contains(p)).collect();
        doc.delete_pages(&delete);
        doc.prune_objects();
        outputs.push(OutputFile {
            name: out_name,
            mime: PDF_MIME,
            bytes: save(doc)?,
        });
    }
    Ok(outputs)
}

/// Rotate every page by `angle` degrees clockwise (90/180/270).
pub fn rotate(name: &str, bytes: &[u8], angle: i32) -> Result<OutputFile, PzError> {
    if !matches!(angle, 90 | 180 | 270) {
        return Err(PzError::Invalid("angle must be 90, 180 or 270".into()));
    }
    let mut doc = load(bytes)?;
    let pages = doc.get_pages();
    for (_, object_id) in pages {
        let current = doc
            .get_object(object_id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(b"Rotate").ok())
            .and_then(|r| r.as_i64().ok())
            .unwrap_or(0);
        let dict = doc
            .get_object_mut(object_id)
            .map_err(|e| PzError::Failed(format!("broken page tree: {e}")))?
            .as_dict_mut()
            .map_err(|e| PzError::Failed(format!("broken page object: {e}")))?;
        dict.set("Rotate", (current + angle as i64).rem_euclid(360));
    }
    Ok(OutputFile {
        name: format!("{}-rotated.pdf", stem(name)),
        mime: PDF_MIME,
        bytes: save(doc)?,
    })
}

/// Losslessly shrink a PDF: recompress content streams and drop unused objects.
pub fn compress(name: &str, bytes: &[u8]) -> Result<OutputFile, PzError> {
    let mut doc = load(bytes)?;
    doc.prune_objects();
    doc.compress();
    let out = save(doc)?;
    // Never hand back a bigger file than the original.
    let (final_bytes, note) = if out.len() < bytes.len() {
        (out, "-compressed")
    } else {
        (bytes.to_vec(), "")
    };
    Ok(OutputFile {
        name: format!("{}{note}.pdf", stem(name)),
        mime: PDF_MIME,
        bytes: final_bytes,
    })
}

/// Build a PDF with one page per JPEG. Each entry is `(width, height,
/// jpeg_bytes)` as produced by `pz_img::to_jpeg`; the image is embedded
/// as-is (DCTDecode) on a page sized to it at 72 dpi.
pub fn from_jpegs(images: &[(u32, u32, Vec<u8>)]) -> Result<Vec<u8>, PzError> {
    if images.is_empty() {
        return Err(PzError::Invalid("select at least one image".into()));
    }
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut kids: Vec<Object> = Vec::new();
    for (i, (w, h, jpeg)) in images.iter().enumerate() {
        let img_id = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => *w,
                "Height" => *h,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
                "Filter" => "DCTDecode",
            },
            jpeg.clone(),
        ));
        let im_name = format!("Im{i}");
        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![
                        (*w as i64).into(),
                        0.into(),
                        0.into(),
                        (*h as i64).into(),
                        0.into(),
                        0.into(),
                    ],
                ),
                Operation::new("Do", vec![Object::Name(im_name.clone().into_bytes())]),
                Operation::new("Q", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            content
                .encode()
                .map_err(|e| PzError::Failed(format!("could not build page: {e}")))?,
        ));
        let mut xobjects = Dictionary::new();
        xobjects.set(im_name, img_id);
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => dictionary! { "XObject" => xobjects },
            "MediaBox" => vec![0.into(), 0.into(), (*w as i64).into(), (*h as i64).into()],
        });
        kids.push(page_id.into());
    }
    let count = kids.len() as u32;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => count,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    // No doc.compress(): the JPEG streams already carry a DCTDecode filter.
    save(doc)
}

/// Stamp `text` diagonally across the middle of every page.
pub fn watermark(name: &str, bytes: &[u8], text: &str) -> Result<OutputFile, PzError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(PzError::Invalid("enter the watermark text".into()));
    }
    const COS45: f64 = std::f64::consts::FRAC_1_SQRT_2;
    let mut doc = load(bytes)?;
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let pages = doc.get_pages();
    for (_, page_id) in pages {
        let (w, h) = page_size(&doc, page_id);
        // Rough centering: Helvetica averages ~0.5 em per glyph.
        let font_size = 48.0;
        let half_width = text.chars().count() as f64 * font_size * 0.25;
        let ops = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new("g", vec![0.75.into()]), // light gray fill
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["PZwm".into(), font_size.into()]),
                // Rotate 45° around the page centre (cos 45° = sin 45° = 1/√2).
                Operation::new(
                    "Tm",
                    vec![
                        COS45.into(),
                        COS45.into(),
                        (-COS45).into(),
                        COS45.into(),
                        (w / 2.0 - half_width * COS45).into(),
                        (h / 2.0 - half_width * COS45).into(),
                    ],
                ),
                Operation::new("Tj", vec![Object::string_literal(text)]),
                Operation::new("ET", vec![]),
                Operation::new("Q", vec![]),
            ],
        };
        let stamp = ops
            .encode()
            .map_err(|e| PzError::Failed(format!("could not build watermark: {e}")))?;

        add_page_font(&mut doc, page_id, "PZwm", font_id)?;

        // Wrap the existing content in q/Q so its graphics state can't skew
        // the stamp, then append the stamp.
        let existing = doc.get_page_content(page_id);
        let mut combined = Vec::with_capacity(existing.len() + stamp.len() + 4);
        combined.extend_from_slice(b"q\n");
        combined.extend_from_slice(&existing);
        combined.extend_from_slice(b"\nQ\n");
        combined.extend_from_slice(&stamp);
        doc.change_page_content(page_id, combined)
            .map_err(|e| PzError::Failed(format!("could not write page content: {e}")))?;
    }
    Ok(OutputFile {
        name: format!("{}-watermarked.pdf", stem(name)),
        mime: PDF_MIME,
        bytes: save(doc)?,
    })
}

/// Lossy Latin-1: Helvetica with the default encoding can't show more.
fn latin1_lossy(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| if (c as u32) < 256 { c as u8 } else { b'?' })
        .collect()
}

/// Page MediaBox size in points, defaulting to US Letter if absent/inherited.
fn page_size(doc: &Document, page_id: ObjectId) -> (f64, f64) {
    let rect = doc
        .get_object(page_id)
        .ok()
        .and_then(|o| o.as_dict().ok())
        .and_then(|d| d.get(b"MediaBox").ok())
        .and_then(|b| doc.dereference(b).ok().map(|(_, o)| o))
        .and_then(|o| o.as_array().ok())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_float().ok())
                .collect::<Vec<f32>>()
        });
    match rect.as_deref() {
        Some([x0, y0, x1, y1]) => ((x1 - x0) as f64, (y1 - y0) as f64),
        _ => (612.0, 792.0),
    }
}

/// Register `font_id` under `res_name` in the page's font resources.
fn add_page_font(
    doc: &mut Document,
    page_id: ObjectId,
    res_name: &str,
    font_id: ObjectId,
) -> Result<(), PzError> {
    add_page_resource(doc, page_id, "Font", res_name, font_id)
}

/// Register `obj_id` under `res_name` in the page's `category` resources
/// ("Font", "XObject", …), following references and creating dictionaries
/// as needed.
fn add_page_resource(
    doc: &mut Document,
    page_id: ObjectId,
    category: &str,
    res_name: &str,
    obj_id: ObjectId,
) -> Result<(), PzError> {
    let broken = |e: lopdf::Error| PzError::Failed(format!("broken page object: {e}"));

    // Resolve where the Resources dictionary lives: its own object (Some(id))
    // or inline on the page (None) — creating an empty one if absent.
    let resources_entry = doc
        .get_object(page_id)
        .map_err(broken)?
        .as_dict()
        .map_err(broken)?
        .get(b"Resources")
        .map(|o| o.to_owned());
    let res_obj: Option<ObjectId> = match resources_entry {
        Ok(Object::Reference(rid)) => Some(rid),
        Ok(Object::Dictionary(_)) => None,
        _ => {
            doc.get_object_mut(page_id)
                .map_err(broken)?
                .as_dict_mut()
                .map_err(broken)?
                .set("Resources", Dictionary::new());
            None
        }
    };

    // The category entry ("Font"/"XObject") may itself be inline or a
    // reference.
    let cat_entry = {
        let resources: &Dictionary = match res_obj {
            Some(rid) => doc
                .get_object(rid)
                .map_err(broken)?
                .as_dict()
                .map_err(broken)?,
            None => doc
                .get_object(page_id)
                .map_err(broken)?
                .as_dict()
                .map_err(broken)?
                .get(b"Resources")
                .map_err(broken)?
                .as_dict()
                .map_err(broken)?,
        };
        resources.get(category.as_bytes()).map(|o| o.to_owned())
    };

    match cat_entry {
        Ok(Object::Reference(cid)) => {
            doc.get_object_mut(cid)
                .map_err(broken)?
                .as_dict_mut()
                .map_err(broken)?
                .set(res_name, obj_id);
        }
        other => {
            let mut entries = match other {
                Ok(Object::Dictionary(d)) => d,
                _ => Dictionary::new(),
            };
            entries.set(res_name, obj_id);
            let resources: &mut Dictionary = match res_obj {
                Some(rid) => doc
                    .get_object_mut(rid)
                    .map_err(broken)?
                    .as_dict_mut()
                    .map_err(broken)?,
                None => doc
                    .get_object_mut(page_id)
                    .map_err(broken)?
                    .as_dict_mut()
                    .map_err(broken)?
                    .get_mut(b"Resources")
                    .map_err(broken)?
                    .as_dict_mut()
                    .map_err(broken)?,
            };
            resources.set(category, entries);
        }
    }
    Ok(())
}

/// Rebuild the page tree in exactly the order given by `order_spec`
/// ("3,1,2"). Pages not listed are dropped; listed twice, duplicated.
pub fn reorder(name: &str, bytes: &[u8], order_spec: &str) -> Result<OutputFile, PzError> {
    let mut doc = load(bytes)?;
    let page_ids = doc.get_pages();
    let total = page_ids.len() as u32;
    if total == 0 {
        return Err(PzError::Failed("document has no pages".into()));
    }
    let order = parse_page_order(order_spec, total)?;

    let keep: BTreeSet<u32> = order.iter().copied().collect();
    let delete: Vec<u32> = (1..=total).filter(|p| !keep.contains(p)).collect();
    doc.delete_pages(&delete);

    let broken = |e: lopdf::Error| PzError::Failed(format!("broken document: {e}"));
    let root_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .map_err(broken)?;
    let pages_root = doc
        .get_object(root_id)
        .map_err(broken)?
        .as_dict()
        .map_err(broken)?
        .get(b"Pages")
        .and_then(|o| o.as_reference())
        .map_err(broken)?;

    // Flatten: every surviving page hangs directly off the root Pages node,
    // in spec order. Intermediate tree nodes become orphans and get pruned.
    for page in order.iter() {
        let pid = page_ids[page];
        doc.get_object_mut(pid)
            .map_err(broken)?
            .as_dict_mut()
            .map_err(broken)?
            .set("Parent", pages_root);
    }
    let kids: Vec<Object> = order
        .iter()
        .map(|p| Object::Reference(page_ids[p]))
        .collect();
    let count = kids.len() as u32;
    let pages_dict = doc
        .get_object_mut(pages_root)
        .map_err(broken)?
        .as_dict_mut()
        .map_err(broken)?;
    pages_dict.set("Kids", kids);
    pages_dict.set("Count", count);

    doc.prune_objects();
    Ok(OutputFile {
        name: format!("{}-reordered.pdf", stem(name)),
        mime: PDF_MIME,
        bytes: save(doc)?,
    })
}

/// Stamp "n / total" centered in the bottom margin of every page.
pub fn page_numbers(name: &str, bytes: &[u8]) -> Result<OutputFile, PzError> {
    let mut doc = load(bytes)?;
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let pages = doc.get_pages();
    let total = pages.len();
    for (num, page_id) in pages {
        let (w, _) = page_size(&doc, page_id);
        let label = format!("{num} / {total}");
        let font_size = 11.0;
        let half_width = label.chars().count() as f64 * font_size * 0.25;
        let ops = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new("g", vec![0.35.into()]),
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["PZpn".into(), font_size.into()]),
                Operation::new("Td", vec![(w / 2.0 - half_width).into(), 22.0.into()]),
                Operation::new("Tj", vec![Object::string_literal(label)]),
                Operation::new("ET", vec![]),
                Operation::new("Q", vec![]),
            ],
        };
        let stamp = ops
            .encode()
            .map_err(|e| PzError::Failed(format!("could not build page number: {e}")))?;
        add_page_font(&mut doc, page_id, "PZpn", font_id)?;
        let existing = doc.get_page_content(page_id);
        let mut combined = Vec::with_capacity(existing.len() + stamp.len() + 4);
        combined.extend_from_slice(b"q\n");
        combined.extend_from_slice(&existing);
        combined.extend_from_slice(b"\nQ\n");
        combined.extend_from_slice(&stamp);
        doc.change_page_content(page_id, combined)
            .map_err(|e| PzError::Failed(format!("could not write page content: {e}")))?;
    }
    Ok(OutputFile {
        name: format!("{}-numbered.pdf", stem(name)),
        mime: PDF_MIME,
        bytes: save(doc)?,
    })
}

/// Trim margins (in PDF points, 72/inch) off every page by shrinking the
/// MediaBox/CropBox.
pub fn crop_margins(
    name: &str,
    bytes: &[u8],
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
) -> Result<OutputFile, PzError> {
    if left == 0 && top == 0 && right == 0 && bottom == 0 {
        return Err(PzError::Invalid("set at least one margin to trim".into()));
    }
    let mut doc = load(bytes)?;
    let broken = |e: lopdf::Error| PzError::Failed(format!("broken page object: {e}"));
    for (_, page_id) in doc.get_pages() {
        let (w, h) = page_size(&doc, page_id);
        let (l, t, r, b) = (left as f64, top as f64, right as f64, bottom as f64);
        if l + r >= w || t + b >= h {
            return Err(PzError::Invalid(format!(
                "margins remove the whole page ({w:.0}x{h:.0} pt)"
            )));
        }
        let new_box = vec![
            Object::Real(l as f32),
            Object::Real(b as f32),
            Object::Real((w - r) as f32),
            Object::Real((h - t) as f32),
        ];
        let dict = doc
            .get_object_mut(page_id)
            .map_err(broken)?
            .as_dict_mut()
            .map_err(broken)?;
        dict.set("MediaBox", new_box.clone());
        dict.set("CropBox", new_box);
    }
    Ok(OutputFile {
        name: format!("{}-cropped.pdf", stem(name)),
        mime: PDF_MIME,
        bytes: save(doc)?,
    })
}

/// Extract all text into a plain .txt file.
pub fn extract_text(name: &str, bytes: &[u8]) -> Result<OutputFile, PzError> {
    let doc = load(bytes)?;
    let pages: Vec<u32> = doc.get_pages().keys().copied().collect();
    if pages.is_empty() {
        return Err(PzError::Failed("document has no pages".into()));
    }
    let text = doc
        .extract_text(&pages)
        .map_err(|e| PzError::Failed(format!("could not extract text: {e}")))?;
    if text.trim().is_empty() {
        return Err(PzError::Unsupported(
            "no extractable text found — the PDF is likely scanned images (OCR isn't supported yet)".into(),
        ));
    }
    Ok(OutputFile {
        name: format!("{}.txt", stem(name)),
        mime: "text/plain",
        bytes: text.into_bytes(),
    })
}

/// Best-effort structural repair: reparse leniently, renumber objects,
/// drop unreferenced garbage, and write a clean xref.
pub fn repair(name: &str, bytes: &[u8]) -> Result<OutputFile, PzError> {
    let mut doc = load(bytes)?;
    if doc.get_pages().is_empty() {
        return Err(PzError::Failed("no recoverable pages found".into()));
    }
    doc.renumber_objects();
    doc.prune_objects();
    Ok(OutputFile {
        name: format!("{}-repaired.pdf", stem(name)),
        mime: PDF_MIME,
        bytes: save(doc)?,
    })
}

/// Remove a known password: decrypt and save an unprotected copy.
pub fn unlock(name: &str, bytes: &[u8], password: &str) -> Result<OutputFile, PzError> {
    if !load(bytes)?.is_encrypted() {
        return Err(PzError::Invalid(
            "this PDF is not password-protected".into(),
        ));
    }
    // Plain load_mem defers object parsing for encrypted files; loading
    // with the password parses + decrypts everything in one pass.
    let doc = Document::load_mem_with_options(bytes, lopdf::LoadOptions::with_password(password))
        .map_err(|_| PzError::Failed("wrong password for this PDF".into()))?;
    if doc.get_pages().is_empty() {
        return Err(PzError::Failed(
            "decrypted PDF has no readable pages".into(),
        ));
    }
    Ok(OutputFile {
        name: format!("{}-unlocked.pdf", stem(name)),
        mime: PDF_MIME,
        bytes: save(doc)?,
    })
}

/// Password-protect with standard PDF AES-256 (PDF 2.0 / R6) encryption —
/// the result opens in any modern PDF viewer. `random` must be at least 64
/// CSPRNG bytes supplied by the caller (32 for the file encryption key,
/// 32 for the file ID); pz-pdf itself stays RNG-free.
pub fn protect(
    name: &str,
    bytes: &[u8],
    password: &str,
    random: &[u8],
) -> Result<OutputFile, PzError> {
    use lopdf::encryption::crypt_filters::{Aes256CryptFilter, CryptFilter};
    use lopdf::{EncryptionState, EncryptionVersion, Permissions};
    use std::collections::BTreeMap;
    use std::sync::Arc;

    if password.is_empty() {
        return Err(PzError::Invalid("enter a password".into()));
    }
    if random.len() < 64 {
        return Err(PzError::Invalid("need at least 64 random bytes".into()));
    }
    let mut doc = load(bytes)?;
    if doc.is_encrypted() {
        return Err(PzError::Invalid("this PDF is already encrypted".into()));
    }
    // A file ID is required plumbing for encrypted PDFs; set one if absent.
    if doc.trailer.get(b"ID").is_err() {
        doc.trailer.set(
            "ID",
            Object::Array(vec![
                Object::string_literal(random[32..48].to_vec()),
                Object::string_literal(random[48..64].to_vec()),
            ]),
        );
    }

    let mut crypt_filters: BTreeMap<Vec<u8>, Arc<dyn CryptFilter>> = BTreeMap::new();
    crypt_filters.insert(b"StdCF".to_vec(), Arc::new(Aes256CryptFilter));
    let version = EncryptionVersion::V5 {
        encrypt_metadata: true,
        crypt_filters,
        file_encryption_key: &random[..32],
        stream_filter: b"StdCF".to_vec(),
        string_filter: b"StdCF".to_vec(),
        owner_password: password,
        user_password: password,
        permissions: Permissions::all(),
    };
    let state = EncryptionState::try_from(version)
        .map_err(|e| PzError::Failed(format!("could not set up encryption: {e}")))?;
    doc.encrypt(&state)
        .map_err(|e| PzError::Failed(format!("could not encrypt PDF: {e}")))?;
    Ok(OutputFile {
        name: format!("{}-protected.pdf", stem(name)),
        mime: PDF_MIME,
        bytes: save(doc)?,
    })
}

/// A freehand ink stroke (signature / handwriting) in PDF coordinates
/// (points, origin bottom-left).
#[derive(Debug, Clone)]
pub struct Stroke {
    pub color: (u8, u8, u8),
    pub width: f32,
    pub points: Vec<(f32, f32)>,
}

/// A JPEG placed on a page. `rect` is (x, y, width, height) in PDF points,
/// with y the *bottom* edge of the image.
#[derive(Debug, Clone)]
pub struct PlacedJpeg {
    pub jpeg: Vec<u8>,
    pub width_px: u32,
    pub height_px: u32,
    pub rect: (f32, f32, f32, f32),
}

/// A typed text box. `pos` is the baseline start of the first line in PDF
/// points (origin bottom-left); newlines produce extra lines at 1.25×
/// leading. Rendered in Helvetica (WinAnsi — non-Latin-1 chars become '?').
#[derive(Debug, Clone)]
pub struct PlacedText {
    pub text: String,
    pub size: f32,
    pub color: (u8, u8, u8),
    pub pos: (f32, f32),
}

/// Everything the editor drew on one page.
#[derive(Debug, Clone, Default)]
pub struct PageEdits {
    /// 1-based page number.
    pub page: u32,
    pub strokes: Vec<Stroke>,
    pub images: Vec<PlacedJpeg>,
    pub texts: Vec<PlacedText>,
}

/// Bake editor annotations (ink strokes and placed images) into the PDF.
/// Images render below ink so signatures stay visible on top of stamps.
pub fn annotate(name: &str, bytes: &[u8], edits: &[PageEdits]) -> Result<OutputFile, PzError> {
    let mut doc = load(bytes)?;
    let pages = doc.get_pages();
    let total = pages.len() as u32;
    let mut img_counter = 0usize;
    let mut applied = false;

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    for edit in edits {
        if edit.strokes.is_empty() && edit.images.is_empty() && edit.texts.is_empty() {
            continue;
        }
        let Some(&page_id) = pages.get(&edit.page) else {
            return Err(PzError::Invalid(format!(
                "page {} is out of range (document has {total} pages)",
                edit.page
            )));
        };
        applied = true;

        let mut ops: Vec<Operation> = Vec::new();
        for img in &edit.images {
            img_counter += 1;
            let res_name = format!("PZim{img_counter}");
            let img_id = doc.add_object(Stream::new(
                dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Image",
                    "Width" => img.width_px,
                    "Height" => img.height_px,
                    "ColorSpace" => "DeviceRGB",
                    "BitsPerComponent" => 8,
                    "Filter" => "DCTDecode",
                },
                img.jpeg.clone(),
            ));
            add_page_resource(&mut doc, page_id, "XObject", &res_name, img_id)?;
            let (x, y, w, h) = img.rect;
            ops.push(Operation::new("q", vec![]));
            ops.push(Operation::new(
                "cm",
                vec![w.into(), 0.into(), 0.into(), h.into(), x.into(), y.into()],
            ));
            ops.push(Operation::new(
                "Do",
                vec![Object::Name(res_name.into_bytes())],
            ));
            ops.push(Operation::new("Q", vec![]));
        }
        if !edit.texts.is_empty() {
            add_page_resource(&mut doc, page_id, "Font", "PZtx", font_id)?;
        }
        for txt in &edit.texts {
            if txt.text.trim().is_empty() {
                continue;
            }
            let (r, g, b) = txt.color;
            let size = txt.size.clamp(4.0, 144.0);
            ops.push(Operation::new("q", vec![]));
            ops.push(Operation::new("BT", vec![]));
            ops.push(Operation::new("Tf", vec!["PZtx".into(), size.into()]));
            ops.push(Operation::new("TL", vec![(size * 1.25).into()]));
            ops.push(Operation::new(
                "rg",
                vec![
                    (r as f32 / 255.0).into(),
                    (g as f32 / 255.0).into(),
                    (b as f32 / 255.0).into(),
                ],
            ));
            ops.push(Operation::new(
                "Td",
                vec![txt.pos.0.into(), txt.pos.1.into()],
            ));
            for (i, line) in txt.text.lines().enumerate() {
                if i > 0 {
                    ops.push(Operation::new("T*", vec![]));
                }
                ops.push(Operation::new(
                    "Tj",
                    vec![Object::String(
                        latin1_lossy(line),
                        lopdf::StringFormat::Literal,
                    )],
                ));
            }
            ops.push(Operation::new("ET", vec![]));
            ops.push(Operation::new("Q", vec![]));
        }
        for stroke in &edit.strokes {
            if stroke.points.is_empty() {
                continue;
            }
            let (r, g, b) = stroke.color;
            ops.push(Operation::new("q", vec![]));
            ops.push(Operation::new("J", vec![1.into()])); // round caps
            ops.push(Operation::new("j", vec![1.into()])); // round joins
            ops.push(Operation::new("w", vec![stroke.width.max(0.2).into()]));
            ops.push(Operation::new(
                "RG",
                vec![
                    (r as f32 / 255.0).into(),
                    (g as f32 / 255.0).into(),
                    (b as f32 / 255.0).into(),
                ],
            ));
            let (x0, y0) = stroke.points[0];
            ops.push(Operation::new("m", vec![x0.into(), y0.into()]));
            if stroke.points.len() == 1 {
                // A tap becomes a dot: zero-length segment with round caps.
                ops.push(Operation::new("l", vec![x0.into(), y0.into()]));
            }
            for (x, y) in stroke.points.iter().skip(1) {
                ops.push(Operation::new("l", vec![(*x).into(), (*y).into()]));
            }
            ops.push(Operation::new("S", vec![]));
            ops.push(Operation::new("Q", vec![]));
        }

        let stamp = Content { operations: ops }
            .encode()
            .map_err(|e| PzError::Failed(format!("could not build annotations: {e}")))?;
        let existing = doc.get_page_content(page_id);
        let mut combined = Vec::with_capacity(existing.len() + stamp.len() + 4);
        combined.extend_from_slice(b"q\n");
        combined.extend_from_slice(&existing);
        combined.extend_from_slice(b"\nQ\n");
        combined.extend_from_slice(&stamp);
        doc.change_page_content(page_id, combined)
            .map_err(|e| PzError::Failed(format!("could not write page content: {e}")))?;
    }

    if !applied {
        return Err(PzError::Invalid(
            "nothing to apply — draw or place something first".into(),
        ));
    }
    doc.prune_objects(); // drops the shared font object if no page used it
    Ok(OutputFile {
        name: format!("{}-edited.pdf", stem(name)),
        mime: PDF_MIME,
        bytes: save(doc)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a small n-page PDF fully in memory.
    fn sample_pdf(pages: usize) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });
        let mut kids: Vec<Object> = Vec::new();
        for i in 0..pages {
            let content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new("Tf", vec!["F1".into(), 24.into()]),
                    Operation::new("Td", vec![100.into(), 600.into()]),
                    Operation::new(
                        "Tj",
                        vec![Object::string_literal(format!("Page {}", i + 1))],
                    ),
                    Operation::new("ET", vec![]),
                ],
            };
            let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => resources_id,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            });
            kids.push(page_id.into());
        }
        let count = kids.len() as u32;
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => kids,
                "Count" => count,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    fn page_count(bytes: &[u8]) -> usize {
        Document::load_mem(bytes).unwrap().get_pages().len()
    }

    #[test]
    fn merges_two_pdfs() {
        let merged = merge(&[
            ("a.pdf".into(), sample_pdf(2)),
            ("b.pdf".into(), sample_pdf(3)),
        ])
        .unwrap();
        assert_eq!(page_count(&merged), 5);
    }

    #[test]
    fn merge_requires_two_files() {
        assert!(merge(&[("a.pdf".into(), sample_pdf(1))]).is_err());
    }

    #[test]
    fn splits_range() {
        let out = split("doc.pdf", &sample_pdf(5), "2-3").unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(page_count(&out[0].bytes), 2);
        assert_eq!(out[0].name, "doc-pages.pdf");
    }

    #[test]
    fn bursts_into_single_pages() {
        let out = split("doc.pdf", &sample_pdf(3), "").unwrap();
        assert_eq!(out.len(), 3);
        for o in &out {
            assert_eq!(page_count(&o.bytes), 1);
        }
    }

    #[test]
    fn split_rejects_out_of_bounds() {
        assert!(split("doc.pdf", &sample_pdf(2), "5").is_err());
    }

    #[test]
    fn rotates_pages() {
        let out = rotate("doc.pdf", &sample_pdf(2), 90).unwrap();
        let doc = Document::load_mem(&out.bytes).unwrap();
        for (_, oid) in doc.get_pages() {
            let rot = doc
                .get_object(oid)
                .unwrap()
                .as_dict()
                .unwrap()
                .get(b"Rotate")
                .unwrap()
                .as_i64()
                .unwrap();
            assert_eq!(rot, 90);
        }
    }

    #[test]
    fn rotate_rejects_bad_angle() {
        assert!(rotate("doc.pdf", &sample_pdf(1), 45).is_err());
    }

    #[test]
    fn compress_roundtrips() {
        let out = compress("doc.pdf", &sample_pdf(3)).unwrap();
        assert_eq!(page_count(&out.bytes), 3);
    }

    /// Tiny valid JPEG via the image crate (kept local: pz-pdf must not
    /// depend on pz-img).
    fn sample_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            w,
            h,
            image::Rgb([200, 60, 60]),
        ));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
        buf.into_inner()
    }

    #[test]
    fn builds_pdf_from_jpegs() {
        let out =
            from_jpegs(&[(40, 30, sample_jpeg(40, 30)), (20, 20, sample_jpeg(20, 20))]).unwrap();
        assert_eq!(page_count(&out), 2);
    }

    #[test]
    fn from_jpegs_requires_input() {
        assert!(from_jpegs(&[]).is_err());
    }

    #[test]
    fn watermarks_every_page() {
        let out = watermark("doc.pdf", &sample_pdf(2), "CONFIDENTIAL").unwrap();
        let doc = Document::load_mem(&out.bytes).unwrap();
        for (_, pid) in doc.get_pages() {
            let content = doc.get_page_content(pid);
            assert!(content.windows(12).any(|w| w == b"CONFIDENTIAL"));
        }
        assert_eq!(out.name, "doc-watermarked.pdf");
    }

    #[test]
    fn watermark_rejects_empty_text() {
        assert!(watermark("doc.pdf", &sample_pdf(1), "  ").is_err());
    }

    #[test]
    fn reorders_pages() {
        let out = reorder("doc.pdf", &sample_pdf(3), "3,1").unwrap();
        let doc = Document::load_mem(&out.bytes).unwrap();
        let pages: Vec<_> = doc.get_pages().into_values().collect();
        assert_eq!(pages.len(), 2);
        let first = doc.get_page_content(pages[0]);
        assert!(first.windows(6).any(|w| w == b"Page 3"));
        let second = doc.get_page_content(pages[1]);
        assert!(second.windows(6).any(|w| w == b"Page 1"));
    }

    #[test]
    fn reorder_duplicates_pages() {
        let out = reorder("doc.pdf", &sample_pdf(2), "1,1,2").unwrap();
        assert_eq!(page_count(&out.bytes), 3);
    }

    #[test]
    fn reorder_rejects_out_of_bounds() {
        assert!(reorder("doc.pdf", &sample_pdf(2), "3").is_err());
    }

    #[test]
    fn numbers_every_page() {
        let out = page_numbers("doc.pdf", &sample_pdf(3)).unwrap();
        let doc = Document::load_mem(&out.bytes).unwrap();
        let pages: Vec<_> = doc.get_pages().into_values().collect();
        let first = doc.get_page_content(pages[0]);
        assert!(first.windows(5).any(|w| w == b"1 / 3"));
        let last = doc.get_page_content(pages[2]);
        assert!(last.windows(5).any(|w| w == b"3 / 3"));
    }

    #[test]
    fn crops_margins() {
        let out = crop_margins("doc.pdf", &sample_pdf(1), 10, 20, 30, 40).unwrap();
        let doc = Document::load_mem(&out.bytes).unwrap();
        let (_, pid) = doc.get_pages().into_iter().next().unwrap();
        let mb = doc
            .get_object(pid)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_float().unwrap())
            .collect::<Vec<f32>>();
        // Original 612x792; left=10, bottom=40, right: 612-30, top: 792-20.
        assert_eq!(mb, vec![10.0, 40.0, 582.0, 772.0]);
    }

    #[test]
    fn crop_rejects_page_eating_margins() {
        assert!(crop_margins("doc.pdf", &sample_pdf(1), 400, 0, 400, 0).is_err());
    }

    #[test]
    fn extracts_text() {
        let out = extract_text("doc.pdf", &sample_pdf(2)).unwrap();
        let text = String::from_utf8(out.bytes).unwrap();
        assert!(text.contains("Page 1"));
        assert!(text.contains("Page 2"));
        assert_eq!(out.name, "doc.txt");
    }

    #[test]
    fn repair_roundtrips() {
        let out = repair("doc.pdf", &sample_pdf(2)).unwrap();
        assert_eq!(page_count(&out.bytes), 2);
        assert_eq!(out.name, "doc-repaired.pdf");
    }

    #[test]
    fn protect_then_unlock_roundtrip() {
        let key = [7u8; 64]; // fixed randomness in tests; engine passes CSPRNG bytes
        let protected = protect("doc.pdf", &sample_pdf(2), "hunter2", &key).unwrap();
        let doc = Document::load_mem(&protected.bytes).unwrap();
        assert!(doc.is_encrypted());

        let unlocked = unlock("doc-protected.pdf", &protected.bytes, "hunter2").unwrap();
        let doc = Document::load_mem(&unlocked.bytes).unwrap();
        assert!(!doc.is_encrypted());
        assert_eq!(doc.get_pages().len(), 2);
    }

    #[test]
    fn unlock_rejects_wrong_password() {
        let key = [9u8; 64];
        let protected = protect("doc.pdf", &sample_pdf(1), "right", &key).unwrap();
        assert!(unlock("doc.pdf", &protected.bytes, "wrong").is_err());
    }

    #[test]
    fn unlock_rejects_unencrypted() {
        assert!(unlock("doc.pdf", &sample_pdf(1), "pw").is_err());
    }

    #[test]
    fn annotates_strokes_and_images() {
        let edits = vec![PageEdits {
            page: 1,
            strokes: vec![Stroke {
                color: (0, 0, 255),
                width: 2.5,
                points: vec![(100.0, 100.0), (150.0, 130.0), (200.0, 100.0)],
            }],
            images: vec![PlacedJpeg {
                jpeg: sample_jpeg(20, 20),
                width_px: 20,
                height_px: 20,
                rect: (300.0, 500.0, 80.0, 80.0),
            }],
            texts: vec![PlacedText {
                text: "Reviewed by QA\nDept. 7".into(),
                size: 14.0,
                color: (200, 30, 30),
                pos: (72.0, 700.0),
            }],
        }];
        let out = annotate("doc.pdf", &sample_pdf(2), &edits).unwrap();
        assert_eq!(out.name, "doc-edited.pdf");
        let doc = Document::load_mem(&out.bytes).unwrap();
        let pages: Vec<_> = doc.get_pages().into_values().collect();
        let content = doc.get_page_content(pages[0]);
        assert!(content.windows(2).any(|w| w == b"RG")); // ink stroke present
        assert!(content.windows(5).any(|w| w == b"PZim1")); // image placed
        assert!(content.windows(14).any(|w| w == b"Reviewed by QA")); // text
        assert!(content.windows(7).any(|w| w == b"Dept. 7")); // second line
                                                              // Page 2 untouched.
        let content2 = doc.get_page_content(pages[1]);
        assert!(!content2.windows(5).any(|w| w == b"PZim1"));
    }

    #[test]
    fn annotate_single_point_becomes_dot() {
        let edits = vec![PageEdits {
            page: 1,
            strokes: vec![Stroke {
                color: (0, 0, 0),
                width: 4.0,
                points: vec![(50.0, 50.0)],
            }],
            images: vec![],
            texts: vec![],
        }];
        assert!(annotate("doc.pdf", &sample_pdf(1), &edits).is_ok());
    }

    #[test]
    fn annotate_rejects_bad_page_and_empty() {
        let edits = vec![PageEdits {
            page: 9,
            strokes: vec![Stroke {
                color: (0, 0, 0),
                width: 1.0,
                points: vec![(1.0, 1.0), (2.0, 2.0)],
            }],
            images: vec![],
            texts: vec![],
        }];
        assert!(annotate("doc.pdf", &sample_pdf(1), &edits).is_err());
        assert!(annotate("doc.pdf", &sample_pdf(1), &[]).is_err());
    }
}
