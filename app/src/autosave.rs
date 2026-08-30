//! Editor autosave: keep the working document across a refresh (ADR-0013).
//!
//! Strictly on-device. The bytes are sealed with AES-256-GCM
//! (`pz_crypto::seal`) before they ever reach storage, and the key is
//! held apart from them, so "Discard" is a crypto-shred rather than a
//! best-effort delete. `app/assets/autosave.js` owns the IndexedDB side.
//!
//! Scope, stated honestly: this saves the *working document* — every
//! operation that has been applied. Ink or text still floating on the
//! canvas since the last bake is not included; the editor bakes before
//! any document operation, so in practice that window is small.

use dioxus::document::eval;
use dioxus::prelude::*;

use crate::save::{fetch_bytes, object_url, revoke_object_url};

pub const AUTOSAVE_JS: Asset = asset!("/assets/autosave.js");

/// Waits for autosave.js (loaded via a `<script>` tag) before calling it.
const WAIT: &str = "for (let i = 0; i < 100 && typeof pzAutosavePeek === 'undefined'; i++) { await new Promise(r => setTimeout(r, 50)); } if (typeof pzAutosavePeek === 'undefined') { throw new Error('autosave script did not load'); }";

/// What's waiting to be restored, if anything.
#[derive(serde::Deserialize, Clone, PartialEq)]
pub struct Saved {
    pub name: String,
    #[serde(rename = "ageSeconds")]
    pub age_seconds: u64,
}

impl Saved {
    /// "4 minutes ago" — deliberately coarse; this is a reassurance, not
    /// a timestamp.
    pub fn age_text(&self) -> String {
        match self.age_seconds {
            s if s < 90 => "a moment ago".to_string(),
            s if s < 3600 => format!("{} minutes ago", s / 60),
            s if s < 7200 => "an hour ago".to_string(),
            s => format!("{} hours ago", s / 3600),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Seal `bytes` and hand them to the browser's IndexedDB. Failures are
/// deliberately silent: autosave must never interrupt editing.
pub async fn save(name: &str, bytes: &[u8]) {
    let key = pz_crypto::random_bytes(32);
    let Ok(sealed) = pz_crypto::seal(&key, bytes) else {
        return;
    };
    let Some(url) = object_url(&sealed, "application/octet-stream") else {
        return;
    };
    let name_json = serde_json::to_string(name).unwrap_or_else(|_| "\"document.pdf\"".into());
    let js = format!(
        "return (async () => {{ {WAIT} return pzAutosaveSave('{url}', {name_json}, '{}'); }})();",
        hex(&key)
    );
    let _ = eval(&js).await;
    revoke_object_url(&url);
}

/// Is there a restorable document? Metadata only.
pub async fn peek() -> Option<Saved> {
    let js = format!("return (async () => {{ {WAIT} return pzAutosavePeek(); }})();");
    let value = eval(&js).await.ok()?;
    serde_json::from_value(value).ok()
}

/// Fetch and unseal the saved document: `(name, bytes)`.
pub async fn load() -> Option<(String, Vec<u8>)> {
    #[derive(serde::Deserialize)]
    struct Loaded {
        url: String,
        key: String,
        name: String,
    }
    let js = format!("return (async () => {{ {WAIT} return pzAutosaveLoad(); }})();");
    let value = eval(&js).await.ok()?;
    let loaded: Loaded = serde_json::from_value(value).ok()?;
    let sealed = fetch_bytes(&loaded.url).await.ok()?;
    revoke_object_url(&loaded.url);
    let key = unhex(&loaded.key)?;
    let bytes = pz_crypto::open(&key, &sealed).ok()?;
    Some((loaded.name, bytes))
}

/// Shred the key, then drop the stored bytes.
pub async fn clear() {
    let js = format!("return (async () => {{ {WAIT} return pzAutosaveClear(); }})();");
    let _ = eval(&js).await;
}
