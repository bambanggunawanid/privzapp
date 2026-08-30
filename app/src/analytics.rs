//! Anonymous page counting (ADR-0012).
//!
//! One same-origin GET per page view — `/gc/count?p=<path>` — proxied by
//! nginx to a self-hosted GoatCounter configured to store ONLY the page
//! path and the visitor's country. No cookies, no IDs, no fingerprint,
//! no third party; the CSP's `connect-src 'self'` still holds. The
//! browser-side gate honors the user's off toggle (localStorage, set on
//! the Privacy page) and the Global Privacy Control / Do-Not-Track
//! signals before a single byte is sent.

/// Fire-and-forget a page view. Safe to call on every render: the JS
/// side dedupes consecutive hits for the same path.
pub fn page_hit(path: &str) {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        use dioxus::prelude::spawn;
        // serde_json escaping so a path can never break out of the string.
        let p = serde_json::to_string(path).unwrap_or_default();
        let js = format!(
            "(function(p){{ try {{ \
               if (localStorage.getItem('pz-analytics') === 'off') return; \
               if (navigator.globalPrivacyControl === true || navigator.doNotTrack === '1') return; \
               if (window.__pzHit === p) return; \
               window.__pzHit = p; \
               fetch('/gc/count?p=' + encodeURIComponent(p), {{ credentials: 'omit', cache: 'no-store', keepalive: true }}).catch(function(){{}}); \
             }} catch (e) {{}} }})({p});"
        );
        spawn(async move {
            let _ = dioxus::document::eval(&js).await;
        });
    }
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    let _ = path;
}
