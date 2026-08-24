//! Privacy-respecting telemetry for PrivZapp.
//!
//! Hard rules, enforced by construction:
//! - **Opt-in only.** `Telemetry::new(false)` (the default) records nothing at
//!   all — not even in memory.
//! - **No PII, ever.** Events cannot carry filenames, file contents, paths,
//!   IPs, or free-form strings. The schema below is the complete list of what
//!   can be recorded.
//! - **Bucketed, not exact.** Sizes and durations are coarse buckets so events
//!   cannot fingerprint a specific document.
//! - **Ephemeral session id.** A random id per app launch (never persisted),
//!   only so events from one session can be de-duplicated server-side.
//!
//! Transport is intentionally not implemented here; events accumulate locally
//! and `export_json()` is the only way out. The app decides if/when to send.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Everything a telemetry event may contain. Adding a field here is a
/// privacy-review-worthy change — keep it enumerable, never free-form.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    /// Tool slug, e.g. "merge-pdf". Comes from the static registry.
    pub tool: String,
    /// Did the operation succeed?
    pub ok: bool,
    /// Coarse total-input-size bucket, e.g. "1-10MB".
    pub size_bucket: &'static str,
    /// Coarse duration bucket, e.g. "<1s".
    pub duration_bucket: &'static str,
    /// "web" | "desktop" | "mobile".
    pub platform: &'static str,
    /// App version, from the build.
    pub app_version: &'static str,
}

pub fn size_bucket(total_bytes: usize) -> &'static str {
    const MB: usize = 1024 * 1024;
    match total_bytes {
        b if b < MB => "<1MB",
        b if b < 10 * MB => "1-10MB",
        b if b < 100 * MB => "10-100MB",
        _ => ">100MB",
    }
}

pub fn duration_bucket(ms: u128) -> &'static str {
    match ms {
        m if m < 1_000 => "<1s",
        m if m < 5_000 => "1-5s",
        m if m < 30_000 => "5-30s",
        _ => ">30s",
    }
}

#[derive(Debug)]
pub struct Telemetry {
    enabled: bool,
    /// Random per-launch id; never persisted, never derived from the device.
    session: String,
    events: Vec<Event>,
}

impl Telemetry {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            session: hex(&pz_crypto::random_bytes(8)),
            events: Vec::new(),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Flipping this on/off is the user's choice in Settings; disabling also
    /// drops anything queued.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.events.clear();
        }
    }

    /// Record one tool run. No-op unless the user opted in.
    pub fn record(&mut self, event: Event) {
        if self.enabled {
            self.events.push(event);
        }
    }

    pub fn pending(&self) -> usize {
        self.events.len()
    }

    /// Export queued events (with the ephemeral session id) and clear the
    /// queue. This is the only exit path for telemetry data.
    pub fn export_json(&mut self) -> String {
        #[derive(Serialize)]
        struct Batch<'a> {
            session: &'a str,
            events: &'a [Event],
        }
        let json = serde_json::to_string(&Batch {
            session: &self.session,
            events: &self.events,
        })
        .unwrap_or_else(|_| "{}".to_string());
        self.events.clear();
        json
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev() -> Event {
        Event {
            tool: "merge-pdf".into(),
            ok: true,
            size_bucket: size_bucket(3 * 1024 * 1024),
            duration_bucket: duration_bucket(420),
            platform: "web",
            app_version: "0.1.0",
        }
    }

    #[test]
    fn disabled_records_nothing() {
        let mut t = Telemetry::new(false);
        t.record(ev());
        assert_eq!(t.pending(), 0);
    }

    #[test]
    fn enabled_records_and_exports() {
        let mut t = Telemetry::new(true);
        t.record(ev());
        assert_eq!(t.pending(), 1);
        let json = t.export_json();
        assert!(json.contains("merge-pdf"));
        assert!(json.contains("1-10MB"));
        assert_eq!(t.pending(), 0);
    }

    #[test]
    fn disabling_drops_queue() {
        let mut t = Telemetry::new(true);
        t.record(ev());
        t.set_enabled(false);
        assert_eq!(t.pending(), 0);
    }

    #[test]
    fn buckets() {
        assert_eq!(size_bucket(10), "<1MB");
        assert_eq!(size_bucket(50 * 1024 * 1024), "10-100MB");
        assert_eq!(duration_bucket(100), "<1s");
        assert_eq!(duration_bucket(60_000), ">30s");
    }
}
