//! The generic tool page: pick files → tweak options → run → download.
//! One component serves every tool; the registry says which options to show.

use dioxus::document::eval;
use dioxus::html::HasFileData;
use dioxus::prelude::*;
use pz_core::seo::seo_for;
use pz_core::{
    human_size, tool_by_slug, InputFile, OptionKind, OutputFile, ToolOptions, ToolPipeline, TOOLS,
};
use pz_img::TARGET_FORMATS;

use pz_core::i18n;

use crate::save::save_file;
use crate::{current_locale, tr, Route};

const DROPDIR_JS: Asset = asset!("/assets/dropdir.js");

/// Browser-displayable image MIME by file extension — used for upload
/// thumbnails so users can see what actually got picked.
fn image_mime(name: &str) -> Option<&'static str> {
    let ext = name.rsplit('.').next()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => return None,
    })
}

#[component]
pub fn ToolPage(slug: String) -> Element {
    // The editor has its own bespoke page; everything else is generic.
    if slug == "edit-pdf" {
        return rsx! {
            crate::pages::EditorPage {}
        };
    }
    let Some(meta) = tool_by_slug(&slug) else {
        return rsx! {
            section { class: "panel",
                h1 { {tr("Tool not found")} }
                p { {tr("That tool doesn't exist (yet?).")} }
            }
        };
    };

    let loc = current_locale();
    let mut files = use_signal(Vec::<InputFile>::new);
    let mut outputs = use_signal(Vec::<OutputFile>::new);
    let mut error = use_signal(String::new);
    let mut notice = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut dragging = use_signal(|| false);

    // Option state; each tool reads only what its registry entry shows.
    let mut quality = use_signal(|| 80u8);
    let mut width = use_signal(String::new);
    let mut height = use_signal(String::new);
    let mut format = use_signal(|| "png".to_string());
    let mut pages_spec = use_signal(String::new);
    let mut angle = use_signal(|| 90i32);
    let mut text = use_signal(String::new);
    let mut off_x = use_signal(String::new);
    let mut off_y = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut scale = use_signal(|| 2u32);
    let mut percent = use_signal(|| 100u32);
    let mut fps = use_signal(|| 12u32);
    let mut trim_start = use_signal(String::new);
    let mut trim_end = use_signal(String::new);
    let mut video_format = use_signal(|| "mp4".to_string());
    let mut audio_format = use_signal(|| "mp3".to_string());
    let mut lang = use_signal(|| "eng".to_string());

    let meta = *meta;

    // Per-file upload thumbnails (blob URLs), aligned with `files`.
    let mut thumbs = use_signal(Vec::<Option<String>>::new);
    let mut add_file = move |name: String, bytes: Vec<u8>| {
        let thumb = image_mime(&name).and_then(|mime| crate::save::object_url(&bytes, mime));
        thumbs.write().push(thumb);
        files.write().push(InputFile { name, bytes });
    };
    let mut remove_file = move |i: usize| {
        files.write().remove(i);
        if i < thumbs.read().len() {
            if let Some(url) = thumbs.write().remove(i) {
                crate::save::revoke_object_url(&url);
            }
        }
    };

    // Live before/after preview for the lossy image tools: re-runs the
    // engine on the *selected* file (click a thumbnail to pick) whenever
    // quality/resolution/format changes. Results are cached in memory per
    // (file, settings) so switching thumbnails back and forth is instant —
    // deliberately NOT persisted: uploads don't survive a refresh either,
    // and the privacy promise is "leave nothing behind".
    let has_preview = matches!(
        meta.slug,
        "compress-img"
            | "convert-img"
            | "flip-img"
            | "upscale-img"
            | "grayscale-img"
            | "blur-img"
            | "watermark-img"
            | "strip-exif"
            | "crop-img"
            | "rotate-img"
    );
    let mut preview_idx = use_signal(|| 0usize);
    let mut preview_url = use_signal(|| Option::<String>::None);
    let mut preview_note = use_signal(String::new);
    // key "{idx}:{quality}:{percent}:{format}" → (blob url, note).
    let mut preview_cache =
        use_signal(std::collections::HashMap::<String, (Option<String>, String)>::new);
    let mut clear_preview_cache = move || {
        for (_, (url, _)) in preview_cache.write().drain() {
            if let Some(u) = url {
                crate::save::revoke_object_url(&u);
            }
        }
    };
    let mut refresh_preview = move || {
        if !has_preview {
            return;
        }
        let idx = preview_idx().min(files.read().len().saturating_sub(1));
        let Some(f) = files.read().get(idx).cloned() else {
            preview_url.set(None);
            preview_note.set(String::new());
            return;
        };
        // Every setting any previewable tool reads goes into the key.
        let key = format!(
            "{:?}",
            (
                idx,
                quality(),
                percent(),
                format(),
                scale(),
                angle(),
                text(),
                width(),
                height(),
                off_x(),
                off_y(),
            )
        );
        if let Some((url, note)) = preview_cache.read().get(&key).cloned() {
            preview_url.set(url);
            preview_note.set(note);
            return;
        }
        let opts = ToolOptions {
            quality: quality(),
            width: width().trim().parse().unwrap_or(0),
            height: height().trim().parse().unwrap_or(0),
            format: format(),
            pages: pages_spec(),
            angle: angle(),
            text: text(),
            x: off_x().trim().parse().unwrap_or(0),
            y: off_y().trim().parse().unwrap_or(0),
            password: password(),
            scale: scale(),
            percent: percent(),
            ..ToolOptions::default()
        };
        spawn(async move {
            match crate::engine::run(meta.slug, vec![f.clone()], &opts).await {
                Ok(out) => {
                    if let Some(o) = out.first() {
                        let saved = 100i64
                            - (o.bytes.len() as i64 * 100)
                                .checked_div(f.bytes.len() as i64)
                                .unwrap_or(100);
                        let note = format!(
                            "{}: {} → {} ({}{}%)",
                            o.name,
                            human_size(f.bytes.len()),
                            human_size(o.bytes.len()),
                            if saved >= 0 { "−" } else { "+" },
                            saved.abs()
                        );
                        // Browsers can't display every output format.
                        let displayable = matches!(
                            o.mime,
                            "image/png" | "image/jpeg" | "image/webp" | "image/gif" | "image/bmp"
                        );
                        let url = if displayable {
                            crate::save::object_url(&o.bytes, o.mime)
                        } else {
                            None
                        };
                        // The cache owns URL lifetimes (revoked on clear).
                        if preview_cache.read().len() >= 60 {
                            clear_preview_cache();
                        }
                        preview_cache
                            .write()
                            .insert(key, (url.clone(), note.clone()));
                        preview_url.set(url);
                        preview_note.set(note);
                    }
                }
                Err(e) => preview_note.set(e),
            }
        });
    };

    // Folder drops (multi-file tools, web/webview): dropdir.js intercepts a
    // directory drop, walks it, and queues [{name, url}] lists; this loop
    // fetches each blob: URL into the same file list the picker feeds.
    use_future(move || async move {
        // wasm only: on desktop/mobile the native fetch stub can't read
        // blob URLs, so the listener must never attach there — folder
        // drops keep their current (inert) native behavior.
        if !meta.multi || !cfg!(target_arch = "wasm32") {
            return;
        }
        #[derive(serde::Deserialize)]
        struct Dropped {
            name: String,
            url: String,
        }
        // The generation counter retires the previous page's loop: dropping
        // the Eval handle does not cancel its JS, and a batch resolved into
        // a dead loop would vanish (and leak its blob URLs). A stale loop
        // re-queues the batch for the live one and exits instead.
        let mut bridge = eval(
            "for (let i = 0; i < 100 && typeof pzDropInit === 'undefined'; i++) { await new Promise(r => setTimeout(r, 50)); } \
             if (typeof pzDropInit === 'undefined') return; \
             pzDropInit(); \
             const D = window.pzDropState; \
             const myGen = (D.gen = (D.gen || 0) + 1); \
             while (true) { \
               const batch = await pzNextDrop(); \
               if (D.gen !== myGen) { D.queue.unshift(batch); return; } \
               dioxus.send(batch); \
             }",
        );
        while let Ok(batch) = bridge.recv::<Vec<Dropped>>().await {
            for f in &batch {
                match crate::save::fetch_bytes(&f.url).await {
                    Ok(bytes) => add_file(f.name.clone(), bytes),
                    Err(e) => error.set(format!("could not read {}: {e}", f.name)),
                }
                crate::save::revoke_object_url(&f.url);
            }
            if !batch.is_empty() {
                clear_preview_cache();
                refresh_preview();
            }
        }
    });

    let run = move |_: Event<MouseData>| {
        let opts = ToolOptions {
            quality: quality(),
            width: width().trim().parse().unwrap_or(0),
            height: height().trim().parse().unwrap_or(0),
            // The format field is shared; each tool reads its own picker.
            format: if meta.options.contains(&OptionKind::VideoFormat) {
                video_format()
            } else if meta.options.contains(&OptionKind::AudioFormat) {
                audio_format()
            } else {
                format()
            },
            pages: pages_spec(),
            angle: angle(),
            text: text(),
            x: off_x().trim().parse().unwrap_or(0),
            y: off_y().trim().parse().unwrap_or(0),
            password: password(),
            scale: scale(),
            percent: percent(),
            fps: fps(),
            trim_start: trim_start(),
            trim_end: trim_end(),
            lang: lang(),
        };
        busy.set(true);
        error.set(String::new());
        notice.set(String::new());
        outputs.set(Vec::new());
        spawn(async move {
            let input = files.read().clone();
            let result = match meta.pipeline {
                ToolPipeline::Engine => crate::engine::run(meta.slug, input, &opts).await,
                // Pages have to be rasterized by the browser before the
                // engine can package them (ADR-0009).
                ToolPipeline::BrowserRender => match input.first() {
                    Some(file) => crate::render::pdf_to_images(file, &opts).await,
                    None => Err("pick a PDF first".to_string()),
                },
                // Video work runs through the bundled ffmpeg.wasm (ADR-0010).
                ToolPipeline::BrowserFfmpeg => match input.first() {
                    Some(file) => crate::video::run_video_tool(file, meta.slug, &opts).await,
                    None => Err("pick a video first".to_string()),
                },
                // Text recognition through tesseract-wasm (ADR-0011).
                ToolPipeline::BrowserOcr => {
                    crate::ocr::run_ocr_tool(&input, meta.slug, &opts).await
                }
            };
            match result {
                Ok(out) => outputs.set(out),
                Err(e) => error.set(e),
            }
            busy.set(false);
        });
    };

    let total_in: usize = files.read().iter().map(|f| f.bytes.len()).sum();
    let seo = seo_for(meta.slug);

    rsx! {
        if let Some(seo) = seo {
            document::Title { "{seo.title}" }
            document::Meta { name: "description", content: seo.description }
        }
        // Only the tools that rasterize pull in the page renderer (and,
        // through it, PDF.js) — every other tool stays wasm-only.
        if meta.pipeline == ToolPipeline::BrowserRender {
            document::Script { src: crate::render::PDFRENDER_JS }
        }
        if meta.pipeline == ToolPipeline::BrowserFfmpeg {
            document::Script { src: crate::video::VIDEOTOOL_JS }
        }
        // OCR pages also mount the (tiny) page renderer: scanned PDFs are
        // rasterized before recognition; PDF.js itself stays lazy.
        if meta.pipeline == ToolPipeline::BrowserOcr {
            document::Script { src: crate::ocr::OCRTOOL_JS }
            document::Script { src: crate::render::PDFRENDER_JS }
        }
        if meta.multi && cfg!(target_arch = "wasm32") {
            document::Script { src: DROPDIR_JS }
        }
        section { class: "tool-head",
            if let Some(src) = crate::icons::tool_icon(meta.slug) {
                img { class: "tool-icon-svg big", src, alt: "" }
            } else {
                div { class: "tool-icon big", {meta.icon} }
            }
            div {
                h1 { {i18n::tool_name(&meta, loc)} }
                p { class: "muted", {i18n::tool_tagline(&meta, loc)} }
            }
        }

        section { class: "panel",
            label {
                class: if dragging() { "dropzone drag" } else { "dropzone" },
                // Opt-in marker for dropdir.js: only dropzones carrying it
                // get folder drops intercepted (single-file tools and the
                // editor must stay untouched).
                "data-dropdir": if meta.multi && cfg!(target_arch = "wasm32") { Some("1") } else { None },
                r#for: "file-in",
                ondragover: move |evt| {
                    evt.prevent_default();
                    dragging.set(true);
                },
                ondragleave: move |evt| {
                    evt.prevent_default();
                    dragging.set(false);
                },
                ondrop: move |evt| {
                    evt.prevent_default();
                    dragging.set(false);
                    spawn(async move {
                        for f in evt.files() {
                            match f.read_bytes().await {
                                Ok(bytes) => add_file(f.name(), bytes.to_vec()),
                                Err(e) => error.set(format!("could not read file: {e}")),
                            }
                        }
                        clear_preview_cache();
                        refresh_preview();
                    });
                },
                span { class: "dz-icon", "⬆" }
                span { class: "dz-label",
                    if meta.multi { {tr("Drop files or a folder here — or click to choose")} }
                    else { {tr("Drop a file here or click to choose")} }
                }
                span { class: "dz-hint", "Files stay on this device — always." }
            }
            input {
                id: "file-in",
                class: "file-input",
                r#type: "file",
                multiple: meta.multi,
                accept: meta.accept,
                onchange: move |evt| {
                    spawn(async move {
                        for f in evt.files() {
                            match f.read_bytes().await {
                                Ok(bytes) => add_file(f.name(), bytes.to_vec()),
                                Err(e) => error.set(format!("could not read file: {e}")),
                            }
                        }
                        clear_preview_cache();
                        refresh_preview();
                    });
                },
            }

            if !files.read().is_empty() {
                if thumbs.read().iter().any(|t| t.is_some()) {
                    // Image uploads: show the pictures themselves so users
                    // can verify what actually got picked.
                    div { class: "thumb-grid",
                        for (i, f) in files.read().iter().enumerate() {
                            div {
                                class: if has_preview && preview_idx().min(files.read().len() - 1) == i { "thumb-card selected" } else { "thumb-card" },
                                key: "{i}-{f.name}",
                                if let Some(Some(url)) = thumbs.read().get(i) {
                                    // Click an image to make it the preview target.
                                    img {
                                        class: "thumb-img",
                                        src: "{url}",
                                        alt: "{f.name}",
                                        onclick: move |_| {
                                            if has_preview {
                                                preview_idx.set(i);
                                                refresh_preview();
                                            }
                                        },
                                    }
                                } else {
                                    div { class: "thumb-img thumb-generic", "📄" }
                                }
                                div { class: "thumb-meta",
                                    span { class: "file-name", "{f.name}" }
                                    span { class: "file-size", {human_size(f.bytes.len())} }
                                }
                                button {
                                    class: "icon-btn thumb-remove",
                                    title: "Remove",
                                    onclick: move |_| {
                                        remove_file(i);
                                        clear_preview_cache();
                                        refresh_preview();
                                    },
                                    "✕"
                                }
                            }
                        }
                    }
                } else {
                    ul { class: "file-list",
                        for (i, f) in files.read().iter().enumerate() {
                            li { key: "{i}-{f.name}",
                                span { class: "file-name", "{f.name}" }
                                span { class: "file-size", {human_size(f.bytes.len())} }
                                button {
                                    class: "icon-btn",
                                    title: "Remove",
                                    onclick: move |_| { remove_file(i); },
                                    "✕"
                                }
                            }
                        }
                    }
                }
                p { class: "muted small", "Total: " {human_size(total_in)} }
            }

            if !meta.options.is_empty() {
                div { class: "options",
                    for opt in meta.options.iter() {
                        match opt {
                            OptionKind::Quality => rsx! {
                                div { class: "opt",
                                    label { "Quality: {quality}" }
                                    div { class: "quality-row",
                                        button {
                                            class: "ed-icon",
                                            title: "Quality −10",
                                            onclick: move |_| {
                                                quality.set(quality().saturating_sub(10).max(10));
                                                refresh_preview();
                                            },
                                            "−"
                                        }
                                        input {
                                            aria_label: "Quality",
                                            r#type: "range",
                                            min: "10",
                                            max: "100",
                                            step: "10",
                                            value: "{quality}",
                                            // While dragging only the label updates —
                                            // the engine is main-thread wasm, so live
                                            // recompression froze the page mid-slide.
                                            oninput: move |evt| {
                                                quality.set(evt.value().parse().unwrap_or(80));
                                            },
                                            // Fires once on release: recompute preview.
                                            onchange: move |evt| {
                                                quality.set(evt.value().parse().unwrap_or(80));
                                                refresh_preview();
                                            },
                                        }
                                        button {
                                            class: "ed-icon",
                                            title: "Quality +10",
                                            onclick: move |_| {
                                                quality.set((quality() + 10).min(100));
                                                refresh_preview();
                                            },
                                            "+"
                                        }
                                    }
                                }
                            },
                            OptionKind::ResolutionPercent => rsx! {
                                div { class: "opt",
                                    label { "Resolution: {percent}% of original" }
                                    div { class: "quality-row",
                                        button {
                                            class: "ed-icon",
                                            title: "Resolution −10%",
                                            onclick: move |_| {
                                                percent.set(percent().saturating_sub(10).max(10));
                                                refresh_preview();
                                            },
                                            "−"
                                        }
                                        input {
                                            aria_label: "Resolution percent",
                                            r#type: "range",
                                            min: "10",
                                            max: "100",
                                            step: "10",
                                            value: "{percent}",
                                            // Label-only while dragging (main-thread
                                            // wasm); recompute once on release.
                                            oninput: move |evt| {
                                                percent.set(evt.value().parse().unwrap_or(100));
                                            },
                                            onchange: move |evt| {
                                                percent.set(evt.value().parse().unwrap_or(100));
                                                refresh_preview();
                                            },
                                        }
                                        button {
                                            class: "ed-icon",
                                            title: "Resolution +10%",
                                            onclick: move |_| {
                                                percent.set((percent() + 10).min(100));
                                                refresh_preview();
                                            },
                                            "+"
                                        }
                                    }
                                }
                            },
                            OptionKind::Dimensions => rsx! {
                                div { class: "opt",
                                    label { {tr("Size (leave one empty to keep aspect ratio)")} }
                                    div { class: "dim-row",
                                        input {
                                            r#type: "number",
                                            placeholder: "Width px",
                                            value: "{width}",
                                            oninput: move |evt| width.set(evt.value()),
                                            onchange: move |_| refresh_preview(),
                                        }
                                        span { "×" }
                                        input {
                                            r#type: "number",
                                            placeholder: "Height px",
                                            value: "{height}",
                                            oninput: move |evt| height.set(evt.value()),
                                            onchange: move |_| refresh_preview(),
                                        }
                                    }
                                }
                            },
                            OptionKind::TargetFormat => rsx! {
                                div { class: "opt",
                                    label { {tr("Convert to")} }
                                    select {
                                        aria_label: "Convert to format",
                                        value: "{format}",
                                        onchange: move |evt| {
                                            format.set(evt.value());
                                            refresh_preview();
                                        },
                                        for f in TARGET_FORMATS {
                                            option { value: *f, selected: format() == *f, {f.to_uppercase()} }
                                        }
                                    }
                                }
                            },
                            OptionKind::PageRange => rsx! {
                                div { class: "opt",
                                    label { {tr("Pages (e.g. 1-3,5 — empty = every page, each as its own file)")} }
                                    input {
                                        r#type: "text",
                                        placeholder: "1-3,5",
                                        value: "{pages_spec}",
                                        oninput: move |evt| pages_spec.set(evt.value()),
                                    }
                                }
                            },
                            OptionKind::RotateAngle => rsx! {
                                div { class: "opt",
                                    label { "Rotate by" }
                                    select {
                                        aria_label: "Rotate by",
                                        onchange: move |evt| {
                                            angle.set(evt.value().parse().unwrap_or(90));
                                            refresh_preview();
                                        },
                                        option { value: "90", "90° clockwise" }
                                        option { value: "180", "180°" }
                                        option { value: "270", "270° clockwise" }
                                    }
                                }
                            },
                            OptionKind::WatermarkText => rsx! {
                                div { class: "opt",
                                    label { "Watermark text" }
                                    input {
                                        r#type: "text",
                                        placeholder: "CONFIDENTIAL",
                                        value: "{text}",
                                        oninput: move |evt| text.set(evt.value()),
                                        onchange: move |_| refresh_preview(),
                                    }
                                }
                            },
                            OptionKind::PageOrder => rsx! {
                                div { class: "opt",
                                    label { "New page order (e.g. 3,1,2 — repeat to duplicate, omit to drop)" }
                                    input {
                                        r#type: "text",
                                        placeholder: "3,1,2",
                                        value: "{pages_spec}",
                                        oninput: move |evt| pages_spec.set(evt.value()),
                                    }
                                }
                            },
                            OptionKind::CropRect => rsx! {
                                div { class: "opt",
                                    label { "Crop rectangle (px, from top-left)" }
                                    div { class: "dim-row",
                                        input {
                                            r#type: "number",
                                            placeholder: "X",
                                            value: "{off_x}",
                                            oninput: move |evt| off_x.set(evt.value()),
                                            onchange: move |_| refresh_preview(),
                                        }
                                        input {
                                            r#type: "number",
                                            placeholder: "Y",
                                            value: "{off_y}",
                                            oninput: move |evt| off_y.set(evt.value()),
                                            onchange: move |_| refresh_preview(),
                                        }
                                        span { "→" }
                                        input {
                                            r#type: "number",
                                            placeholder: "Width px",
                                            value: "{width}",
                                            oninput: move |evt| width.set(evt.value()),
                                            onchange: move |_| refresh_preview(),
                                        }
                                        span { "×" }
                                        input {
                                            r#type: "number",
                                            placeholder: "Height px",
                                            value: "{height}",
                                            oninput: move |evt| height.set(evt.value()),
                                            onchange: move |_| refresh_preview(),
                                        }
                                    }
                                }
                            },
                            OptionKind::Password => rsx! {
                                div { class: "opt",
                                    label { "Password (used on this device only — if you lose it, the file is gone)" }
                                    input {
                                        r#type: "password",
                                        placeholder: "••••••••",
                                        value: "{password}",
                                        oninput: move |evt| password.set(evt.value()),
                                    }
                                }
                            },
                            OptionKind::Margins => rsx! {
                                div { class: "opt",
                                    label { "Margins to trim (PDF points, 72 = 1 inch)" }
                                    div { class: "dim-row",
                                        input {
                                            r#type: "number",
                                            placeholder: "Left",
                                            value: "{off_x}",
                                            oninput: move |evt| off_x.set(evt.value()),
                                        }
                                        input {
                                            r#type: "number",
                                            placeholder: "Top",
                                            value: "{off_y}",
                                            oninput: move |evt| off_y.set(evt.value()),
                                        }
                                        input {
                                            r#type: "number",
                                            placeholder: "Right",
                                            value: "{width}",
                                            oninput: move |evt| width.set(evt.value()),
                                        }
                                        input {
                                            r#type: "number",
                                            placeholder: "Bottom",
                                            value: "{height}",
                                            oninput: move |evt| height.set(evt.value()),
                                        }
                                    }
                                }
                            },
                            OptionKind::FlipDir => rsx! {
                                div { class: "opt",
                                    label { "Mirror direction" }
                                    select {
                                        aria_label: "Mirror direction",
                                        onchange: move |evt| {
                                            format.set(evt.value());
                                            refresh_preview();
                                        },
                                        option { value: "horizontal", "Horizontal (left ↔ right)" }
                                        option { value: "vertical", "Vertical (top ↕ bottom)" }
                                    }
                                }
                            },
                            OptionKind::ScaleFactor => rsx! {
                                div { class: "opt",
                                    label { {tr("Upscale factor")} }
                                    select {
                                        aria_label: "Upscale factor",
                                        onchange: move |evt| {
                                            scale.set(evt.value().parse().unwrap_or(2));
                                            refresh_preview();
                                        },
                                        option { value: "2", "2× (double size)" }
                                        option { value: "4", "4× (quadruple size)" }
                                    }
                                }
                            },
                            OptionKind::RasterFormat => rsx! {
                                div { class: "opt",
                                    label { {tr("Image format")} }
                                    select {
                                        aria_label: "Image format",
                                        value: "{format}",
                                        onchange: move |evt| format.set(evt.value()),
                                        option { value: "png", selected: format() == "png", "PNG (sharp text, lossless)" }
                                        option { value: "jpg", selected: format() == "jpg", "JPG (smallest, photos)" }
                                        option { value: "webp", selected: format() == "webp", "WebP (modern, small)" }
                                    }
                                }
                            },
                            OptionKind::RenderScale => rsx! {
                                div { class: "opt",
                                    label { {tr("Resolution")} }
                                    select {
                                        aria_label: "Resolution",
                                        onchange: move |evt| scale.set(evt.value().parse().unwrap_or(2)),
                                        option { value: "1", selected: scale() == 1, "1× — 72 DPI (screen preview)" }
                                        option { value: "2", selected: scale() == 2, "2× — 144 DPI (default)" }
                                        option { value: "3", selected: scale() == 3, "3× — 216 DPI" }
                                        option { value: "4", selected: scale() == 4, "4× — 288 DPI (print/OCR)" }
                                    }
                                }
                            },
                            OptionKind::VideoFormat => rsx! {
                                div { class: "opt",
                                    label { {tr("Convert to")} }
                                    select {
                                        aria_label: "Convert to format",
                                        onchange: move |evt| video_format.set(evt.value()),
                                        option { value: "mp4", selected: video_format() == "mp4", "MP4 (H.264 — plays everywhere)" }
                                        option { value: "webm", selected: video_format() == "webm", "WebM (VP8 — royalty-free)" }
                                        option { value: "mkv", selected: video_format() == "mkv", "MKV (Matroska)" }
                                        option { value: "mov", selected: video_format() == "mov", "MOV (QuickTime)" }
                                        option { value: "avi", selected: video_format() == "avi", "AVI (legacy compatibility)" }
                                    }
                                }
                            },
                            OptionKind::AudioFormat => rsx! {
                                div { class: "opt",
                                    label { {tr("Audio format")} }
                                    select {
                                        aria_label: "Audio format",
                                        onchange: move |evt| audio_format.set(evt.value()),
                                        option { value: "mp3", selected: audio_format() == "mp3", "MP3 (plays everywhere)" }
                                        option { value: "wav", selected: audio_format() == "wav", "WAV (uncompressed, lossless)" }
                                        option { value: "ogg", selected: audio_format() == "ogg", "OGG (Vorbis)" }
                                        option { value: "m4a", selected: audio_format() == "m4a", "M4A (AAC)" }
                                    }
                                }
                            },
                            OptionKind::OcrLanguage => rsx! {
                                div { class: "opt",
                                    label { {tr("Language")} }
                                    select {
                                        aria_label: "Recognition language",
                                        onchange: move |evt| lang.set(evt.value()),
                                        option { value: "eng", selected: lang() == "eng", "English" }
                                        option { value: "ind", selected: lang() == "ind", "Indonesian (Bahasa)" }
                                    }
                                }
                            },
                            OptionKind::Fps => rsx! {
                                div { class: "opt",
                                    label { {tr("Frame rate")} }
                                    select {
                                        aria_label: "Frame rate",
                                        onchange: move |evt| fps.set(evt.value().parse().unwrap_or(12)),
                                        option { value: "5", selected: fps() == 5, "5 fps (tiny file)" }
                                        option { value: "10", selected: fps() == 10, "10 fps" }
                                        option { value: "12", selected: fps() == 12, "12 fps (default)" }
                                        option { value: "15", selected: fps() == 15, "15 fps" }
                                        option { value: "24", selected: fps() == 24, "24 fps (smooth, big)" }
                                    }
                                }
                            },
                            OptionKind::TimeRange => rsx! {
                                div { class: "opt",
                                    label { {tr("Start (e.g. 0:05 — empty = from the beginning)")} }
                                    input {
                                        r#type: "text",
                                        aria_label: "Start time",
                                        placeholder: "0:00",
                                        value: "{trim_start}",
                                        oninput: move |evt| trim_start.set(evt.value()),
                                    }
                                }
                                div { class: "opt",
                                    label { {tr("End (e.g. 0:15 — empty = to the end)")} }
                                    input {
                                        r#type: "text",
                                        aria_label: "End time",
                                        placeholder: "0:10",
                                        value: "{trim_end}",
                                        oninput: move |evt| trim_end.set(evt.value()),
                                    }
                                }
                            },
                            OptionKind::Strength => rsx! {
                                div { class: "opt",
                                    label { "Strength: {quality}" }
                                    input {
                                        aria_label: "Strength",
                                        r#type: "range",
                                        min: "1",
                                        max: "100",
                                        value: "{quality}",
                                        oninput: move |evt| quality.set(evt.value().parse().unwrap_or(50)),
                                        onchange: move |evt| {
                                            quality.set(evt.value().parse().unwrap_or(50));
                                            refresh_preview();
                                        },
                                    }
                                }
                            },
                            OptionKind::NamePattern => rsx! {
                                div { class: "opt",
                                    label { "New name pattern — {{n}} becomes 1, 2, 3…" }
                                    input {
                                        r#type: "text",
                                        placeholder: "vacation-{{n}}",
                                        value: "{text}",
                                        oninput: move |evt| text.set(evt.value()),
                                    }
                                }
                            },
                        }
                    }
                }
            }

            if has_preview && !preview_note.read().is_empty() {
                // Side-by-side comparison of the selected image.
                div { class: "preview",
                    div { class: "preview-compare",
                        div { class: "preview-pane",
                            span { class: "muted small",
                                "Original — "
                                {
                                    let idx = preview_idx().min(files.read().len().saturating_sub(1));
                                    files.read().get(idx).map(|f| human_size(f.bytes.len())).unwrap_or_default()
                                }
                            }
                            if let Some(Some(url)) = thumbs.read().get(preview_idx().min(thumbs.read().len().saturating_sub(1))) {
                                img { class: "preview-before", src: "{url}", alt: "Original" }
                            } else {
                                div { class: "thumb-img thumb-generic", "📄" }
                            }
                        }
                        div { class: "preview-pane",
                            span { class: "muted small", "After (live)" }
                            if let Some(url) = preview_url() {
                                img { class: "preview-after", src: "{url}", alt: "Result preview" }
                            }
                        }
                    }
                    p { class: "muted small", "{preview_note}" }
                }
            }

            div { class: "actions",
                button {
                    class: "primary",
                    disabled: busy() || files.read().len() < meta.min_files,
                    onclick: run,
                    if busy() { {tr("Working…")} } else { {meta.name} }
                }
                if !files.read().is_empty() {
                    button {
                        class: "ghost",
                        onclick: move |_| {
                            files.set(Vec::new());
                            for url in thumbs.write().drain(..).flatten() {
                                crate::save::revoke_object_url(&url);
                            }
                            outputs.set(Vec::new());
                            error.set(String::new());
                            notice.set(String::new());
                            clear_preview_cache();
                            refresh_preview();
                        },
                        "Clear"
                    }
                }
            }

            if !error.read().is_empty() {
                p { class: "error", "{error}" }
            }
            if !notice.read().is_empty() {
                p { class: "notice", "{notice}" }
            }
        }

        if !outputs.read().is_empty() {
            section { class: "panel results",
                h2 { {tr("Done ✅")} }
                ul { class: "file-list",
                    for out in outputs() {
                        li { key: "{out.name}",
                            span { class: "file-name", "{out.name}" }
                            span { class: "file-size", {human_size(out.bytes.len())} }
                            button {
                                class: "primary small-btn",
                                onclick: move |_| {
                                    match save_file(&out) {
                                        Ok(Some(path)) => notice.set(format!("Saved to {path}")),
                                        Ok(None) => {}
                                        Err(e) => error.set(e),
                                    }
                                },
                                "Download"
                            }
                        }
                    }
                }
                p { class: "muted small",
                    "Processed on your device in this tab. Nothing was uploaded."
                }
            }
        }

        if let Some(seo) = seo {
            section { class: "panel tool-info",
                p { class: "muted", "{seo.description}" }
                h2 { "Frequently asked questions" }
                for (q , a) in seo.faq.iter() {
                    details { class: "faq",
                        summary { "{q}" }
                        p { "{a}" }
                    }
                }
                h2 { "More " {meta.category.label()} " tools" }
                nav { class: "related",
                    for other in TOOLS.iter().filter(|t| t.category == meta.category && t.slug != meta.slug) {
                        Link {
                            class: "related-link",
                            to: Route::ToolPage { slug: other.slug.to_string() },
                            if let Some(src) = crate::icons::tool_icon(other.slug) {
                                img { class: "rel-ico-svg", src, alt: "", loading: "lazy" }
                            } else {
                                {other.icon}
                                " "
                            }
                            {i18n::tool_name(other, loc)}
                        }
                    }
                }
            }
        }
    }
}
