//! Browser-side video processing through the bundled ffmpeg.wasm
//! (ADR-0010).
//!
//! Video encode/decode is C-library territory (x264, libvpx) that can
//! never live in the pure engine crates (ADR-0002). The compiled FFmpeg
//! runs in a Web Worker, loaded lazily from /ffmpeg/ the first time a
//! video tool runs; this module builds the argument lists — all from
//! typed values, never from raw user strings — and names the output.
//! `pz_engine::run` refuses these slugs; the app dispatches here on
//! `ToolPipeline::BrowserFfmpeg`.

use dioxus::document::eval;
use dioxus::prelude::*;
use pz_core::{parse_timecode, stem, InputFile, OutputFile, ToolOptions};

use crate::save::{object_url, revoke_object_url};

pub const VIDEOTOOL_JS: Asset = asset!("/assets/videotool.js");

/// Waits for videotool.js (loaded via a `<script>` tag) before calling it.
const WAIT_FOR_SCRIPT: &str = "for (let i = 0; i < 100 && typeof pzVidInit === 'undefined'; i++) { await new Promise(r => setTimeout(r, 50)); } if (typeof pzVidInit === 'undefined') { throw new Error('video engine did not load'); }";

/// Download MIME for a video container extension.
fn video_mime(ext: &str) -> &'static str {
    match ext {
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "gif" => "image/gif",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        _ => "application/octet-stream",
    }
}

/// The input's extension, reduced to something safe to splice into a
/// filename inside the ffmpeg FS (the demuxer probes content anyway).
fn safe_ext(name: &str) -> String {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if !ext.is_empty() && ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        ext
    } else {
        "dat".to_string()
    }
}

/// Optional trim window from the options: `(start, duration)` seconds.
fn trim_window(opts: &ToolOptions) -> Result<(Option<f64>, Option<f64>), String> {
    let start = match opts.trim_start.trim() {
        "" => None,
        s => Some(parse_timecode(s).map_err(|e| e.to_string())?),
    };
    let end = match opts.trim_end.trim() {
        "" => None,
        s => Some(parse_timecode(s).map_err(|e| e.to_string())?),
    };
    if let (Some(s), Some(e)) = (start, end) {
        if e <= s {
            return Err("the end time must come after the start time".to_string());
        }
    }
    Ok((start, end))
}

/// One ffmpeg invocation per element; the GIF palette recipe needs two.
struct Plan {
    arg_sets: Vec<Vec<String>>,
    out_fs: String,
    out_name: String,
    scratch: Vec<String>,
}

fn plan(slug: &str, in_name: &str, in_fs: &str, opts: &ToolOptions) -> Result<Plan, String> {
    let base = stem(in_name);
    let (start, end) = trim_window(opts)?;
    let mut input = vec!["-y".to_string()];
    if let Some(s) = start {
        input.extend(["-ss".into(), format!("{s}")]);
    }

    match slug {
        "video-to-gif" => {
            // -to before -i limits the input read; with -ss it still means
            // absolute time, which is what the widget promises.
            if let Some(e) = end {
                input.extend(["-to".into(), format!("{e}")]);
            }
            let fps = opts.fps.clamp(1, 30);
            let mut filters = format!("fps={fps}");
            match (opts.width, opts.height) {
                (0, 0) => {}
                (w, 0) => filters.push_str(&format!(",scale={w}:-1:flags=lanczos")),
                (0, h) => filters.push_str(&format!(",scale=-1:{h}:flags=lanczos")),
                (w, h) => filters.push_str(&format!(",scale={w}:{h}:flags=lanczos")),
            }
            // Two passes: a generated palette beats the fixed 256-color web
            // palette by a mile on photographic clips.
            let mut pass1 = input.clone();
            pass1.extend([
                "-i".into(),
                in_fs.into(),
                "-vf".into(),
                format!("{filters},palettegen"),
                "palette.png".into(),
            ]);
            let mut pass2 = input;
            pass2.extend([
                "-i".into(),
                in_fs.into(),
                "-i".into(),
                "palette.png".into(),
                "-lavfi".into(),
                format!("{filters}[x];[x][1:v]paletteuse"),
                "out.gif".into(),
            ]);
            Ok(Plan {
                arg_sets: vec![pass1, pass2],
                out_fs: "out.gif".into(),
                out_name: format!("{base}.gif"),
                scratch: vec!["palette.png".into()],
            })
        }
        "trim-video" => {
            if start.is_none() && end.is_none() {
                return Err("set a start time, an end time, or both".to_string());
            }
            if let Some(e) = end {
                input.extend(["-to".into(), format!("{e}")]);
            }
            let ext = safe_ext(in_name);
            let out_fs = format!("out.{ext}");
            // Stream copy: lossless and near-instant. The cut snaps to the
            // keyframe before the start time — documented in the FAQ.
            let mut args = input;
            args.extend([
                "-i".into(),
                in_fs.into(),
                "-c".into(),
                "copy".into(),
                out_fs.clone(),
            ]);
            Ok(Plan {
                arg_sets: vec![args],
                out_fs,
                out_name: format!("{base}-trimmed.{ext}"),
                scratch: vec![],
            })
        }
        "convert-video" => {
            let q = u32::from(opts.quality.clamp(1, 100));
            // x264-based containers share the same video args.
            let crf_x264 = (18 + (100 - q) * 14 / 100).to_string();
            let x264: Vec<String> = [
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                &crf_x264,
                "-pix_fmt",
                "yuv420p",
                // x264 requires even dimensions; phones love odd ones.
                "-vf",
                "scale=trunc(iw/2)*2:trunc(ih/2)*2",
            ]
            .map(String::from)
            .to_vec();
            let (fmt, codec_args): (&str, Vec<String>) = match opts.format.as_str() {
                "webm" => {
                    let crf = 10 + (100 - q) * 25 / 100;
                    (
                        "webm",
                        vec![
                            "-c:v".into(),
                            "libvpx".into(),
                            "-crf".into(),
                            crf.to_string(),
                            "-b:v".into(),
                            "0".into(),
                            "-c:a".into(),
                            "libopus".into(),
                        ],
                    )
                }
                "mkv" => {
                    let mut a = x264;
                    a.extend(["-c:a".into(), "aac".into()]);
                    ("mkv", a)
                }
                "mov" => {
                    let mut a = x264;
                    a.extend([
                        "-movflags".into(),
                        "+faststart".into(),
                        "-c:a".into(),
                        "aac".into(),
                    ]);
                    ("mov", a)
                }
                // Legacy compatibility container: MPEG-4 part 2 + MP3.
                "avi" => {
                    let qv = 2 + (100 - q) * 13 / 100;
                    (
                        "avi",
                        vec![
                            "-c:v".into(),
                            "mpeg4".into(),
                            "-q:v".into(),
                            qv.to_string(),
                            "-c:a".into(),
                            "libmp3lame".into(),
                        ],
                    )
                }
                _ => {
                    let mut a = x264;
                    a.extend([
                        "-movflags".into(),
                        "+faststart".into(),
                        "-c:a".into(),
                        "aac".into(),
                    ]);
                    ("mp4", a)
                }
            };
            let out_fs = format!("out.{fmt}");
            let mut args = vec!["-y".to_string(), "-i".into(), in_fs.into()];
            args.extend(codec_args);
            args.push(out_fs.clone());
            let out_name = if safe_ext(in_name) == fmt {
                format!("{base}-converted.{fmt}")
            } else {
                format!("{base}.{fmt}")
            };
            Ok(Plan {
                arg_sets: vec![args],
                out_fs,
                out_name,
                scratch: vec![],
            })
        }
        "extract-audio" => {
            let q = u32::from(opts.quality.clamp(1, 100));
            if let Some(e) = end {
                input.extend(["-to".into(), format!("{e}")]);
            }
            let (fmt, codec_args): (&str, Vec<String>) = match opts.format.as_str() {
                // Lossless; the quality slider has nothing to control.
                "wav" => ("wav", vec!["-c:a".into(), "pcm_s16le".into()]),
                "ogg" => {
                    // libvorbis -q:a runs 0..10.
                    let qv = q / 10;
                    (
                        "ogg",
                        vec![
                            "-c:a".into(),
                            "libvorbis".into(),
                            "-q:a".into(),
                            qv.to_string(),
                        ],
                    )
                }
                "m4a" => (
                    "m4a",
                    vec![
                        "-c:a".into(),
                        "aac".into(),
                        "-b:a".into(),
                        format!("{}k", 64 + q),
                    ],
                ),
                _ => {
                    // LAME VBR -q:a runs 9 (worst) .. 0 (best).
                    let qa = (100 - q) * 9 / 100;
                    (
                        "mp3",
                        vec![
                            "-c:a".into(),
                            "libmp3lame".into(),
                            "-q:a".into(),
                            qa.to_string(),
                        ],
                    )
                }
            };
            let out_fs = format!("out.{fmt}");
            let mut args = input;
            args.extend(["-i".into(), in_fs.into(), "-vn".into()]);
            args.extend(codec_args);
            args.push(out_fs.clone());
            Ok(Plan {
                arg_sets: vec![args],
                out_fs,
                out_name: format!("{base}.{fmt}"),
                scratch: vec![],
            })
        }
        other => Err(format!("\"{other}\" is not a video tool")),
    }
}

/// Run a `ToolPipeline::BrowserFfmpeg` tool end to end.
pub async fn run_video_tool(
    file: &InputFile,
    slug: &str,
    opts: &ToolOptions,
) -> Result<Vec<OutputFile>, String> {
    let in_fs = format!("in.{}", safe_ext(&file.name));
    let p = plan(slug, &file.name, &in_fs, opts)?;

    let url = object_url(&file.bytes, "application/octet-stream");
    let input_arg = match &url {
        Some(u) => u.clone(),
        None => {
            use base64::engine::general_purpose::STANDARD as B64;
            use base64::Engine;
            format!("b64:{}", B64.encode(&file.bytes))
        }
    };
    let args_json = serde_json::to_string(&p.arg_sets).map_err(|e| e.to_string())?;
    let scratch_json = serde_json::to_string(&p.scratch).map_err(|e| e.to_string())?;
    let js = format!(
        "return (async () => {{ {WAIT_FOR_SCRIPT} await pzVidInit(); return pzVidRun('{input_arg}', '{in_fs}', {args_json}, '{out_fs}', {scratch_json}); }})();",
        out_fs = p.out_fs,
    );

    let value = eval(&js).await;
    if let Some(u) = &url {
        revoke_object_url(u);
    }
    let value = value.map_err(|e| format!("video processing failed: {e:?}"))?;
    let b64: String =
        serde_json::from_value(value).map_err(|e| format!("unexpected result: {e}"))?;
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    let bytes = B64.decode(b64).map_err(|e| e.to_string())?;
    let ext = p.out_name.rsplit('.').next().unwrap_or("").to_string();
    Ok(vec![OutputFile {
        name: p.out_name,
        mime: video_mime(&ext),
        bytes,
    }])
}
