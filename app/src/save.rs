//! Handing results back to the user, per platform.
//!
//! Web: a Blob + temporary object URL triggers a normal browser download —
//! the bytes go straight from WASM memory to the user's disk.
//! Native: written to ~/Downloads/PrivZapp (returned so the UI can show it).

use pz_core::OutputFile;

/// Byte buffer → temporary `blob:` object URL (web only). The cheap way to
/// hand large data to JS (PDF.js, `<img>` previews) without serializing it
/// through eval strings. Pair with [`revoke_object_url`].
#[cfg(target_arch = "wasm32")]
pub fn object_url(bytes: &[u8], mime: &str) -> Option<String> {
    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::of1(&array);
    let props = web_sys::BlobPropertyBag::new();
    props.set_type(mime);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &props).ok()?;
    web_sys::Url::create_object_url_with_blob(&blob).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn object_url(_bytes: &[u8], _mime: &str) -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
pub fn revoke_object_url(url: &str) {
    let _ = web_sys::Url::revoke_object_url(url);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn revoke_object_url(_url: &str) {}

#[cfg(target_arch = "wasm32")]
pub fn save_file(file: &OutputFile) -> Result<Option<String>, String> {
    use wasm_bindgen::JsCast;

    let url = object_url(&file.bytes, file.mime).ok_or("could not create download link")?;

    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?;
    let anchor: web_sys::HtmlAnchorElement = document
        .create_element("a")
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|_| "not an anchor".to_string())?;
    anchor.set_href(&url);
    anchor.set_download(&file.name);
    anchor.click();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(None)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_file(file: &OutputFile) -> Result<Option<String>, String> {
    // Belt-and-braces: engine output names are already sanitized, but never
    // let a name traverse out of the target directory.
    let safe_name: String = file
        .name
        .replace(['/', '\\'], "_")
        .trim_start_matches('.')
        .to_string();
    let safe_name = if safe_name.is_empty() {
        "output".to_string()
    } else {
        safe_name
    };

    let dir = dirs::download_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("PrivZapp");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create {}: {e}", dir.display()))?;

    // Don't clobber earlier results.
    let mut path = dir.join(&safe_name);
    let (base, ext) = match safe_name.rsplit_once('.') {
        Some((b, e)) if !b.is_empty() => (b.to_string(), format!(".{e}")),
        _ => (safe_name.clone(), String::new()),
    };
    let mut n = 1u32;
    while path.exists() {
        path = dir.join(format!("{base}-{n}{ext}"));
        n += 1;
    }

    std::fs::write(&path, &file.bytes).map_err(|e| format!("could not save: {e}"))?;
    Ok(Some(path.display().to_string()))
}
