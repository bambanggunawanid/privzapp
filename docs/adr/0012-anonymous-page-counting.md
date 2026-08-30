# ADR-0012: Anonymous page counting via self-hosted GoatCounter

- **Status**: Accepted
- **Date**: 2026-08-30

## Context

The owner needs traffic visibility — which pages get hit, from which
countries, how the site performs — and initially named Google Analytics,
with a disclosure promising "no IP saved, no device name, just country
and page hits."

Those two halves are incompatible. With GA4 the visitor's browser
connects to Google, so the IP is transmitted to Google by definition
(they use it for geolocation under their own terms); device model, OS,
browser and screen data are collected by default; the EU requires a
consent banner; and the integration needs third-party script and
connect targets, which breaks `connect-src 'self'` (the directive
ADR-0008 calls load-bearing), the bundled-never-CDN rule, and the
product's literal "no tracking" claims. It would also under-count this
audience badly — privacy-conscious users run ad blockers.

The owner's actual specification — country + page hits, nothing about
the person — is exactly what self-hosted privacy analytics do.

## Decision

**Self-hosted GoatCounter** (EUPL-1.2, single binary + SQLite) as a
sidecar container, with the collection contract enforced structurally:

1. **The beacon sends the page path and nothing else.** One same-origin
   GET to `/gc/count?p=<path>` per page view (`app/src/analytics.rs`;
   the payload's key list is pinned by a UI test). nginx proxies it to
   the sidecar — the browser never talks to another origin, so the CSP
   stays `connect-src 'self'` unchanged. Single-container runs get a
   baked-in 204 stub instead (`deploy/gc-stub.conf`); docker-compose
   mounts the real proxy (`deploy/gc-proxy.conf`) over it.
2. **The sidecar stores path + country only.** Its entrypoint sets
   GoatCounter's collection bitmask to Location-country alone — on
   every boot, so a dashboard click can't silently widen it. That
   disables sessions/uniques entirely (the daily IP+UA hash is never
   computed), user-agent/browser/OS stats, screen size, language,
   referrer and sub-country regions. The IP is used in memory for the
   country lookup (GoatCounter bundles GeoLite2) and never written;
   nginx `access_log` remains off.
3. **On by default, honestly disclosed, easy to kill.** The Privacy
   page carries the exhaustive list (sent / stored / not-stored) and a
   persistent off toggle; the beacon also honors the Global Privacy
   Control and Do-Not-Track browser signals automatically. No cookie
   banner is needed because nothing identifying is stored or read.
4. **No third party, pinned like everything else.** The sidecar image
   builds from a version- and sha256-pinned release binary
   (`deploy/goatcounter/Dockerfile`). Dashboard on 127.0.0.1:8091 (put
   behind an owner subdomain, e.g. stats.privzapp.com, to expose it —
   GoatCounter's "public" site mode can make it a public stats page).

Google Analytics was rejected, not postponed: no truthful disclosure
matching the owner's stated privacy promise could be written for it.

## Consequences

- The footer promise changed from "no tracking" to "no ads" plus a
  "What we count →" link; the prerendered SEO copy now says "no ads, no
  third-party trackers". The copy stays literally true.
- `pz-telemetry` (per-tool, opt-in event metrics) remains dormant and
  is unaffected; the Privacy page states that plainly.
- Offline/PWA behavior is untouched — an unsendable beacon fails
  silently and nothing depends on it.
- Counting is per-view only: without sessions there are deliberately no
  "unique visitor" numbers. That is the trade the disclosure sells.
- Native (desktop/mobile) builds send nothing — the beacon compiles to
  a no-op off wasm32/web.
- Operational: nginx resolves the sidecar hostname once at startup, so
  recreating only the analytics container makes beacons 504 until the
  web container restarts. Harmless for visitors (fire-and-forget), but
  it silently stops counting — restart both, which `docker compose up
  -d` does anyway.
- The `collect` bitmask is version-sensitive: these are `iota` bitflags
  in GoatCounter's `settings.go` (v2.7.0: Location=16, Region=32,
  Session=128), and the first cut of this ADR shipped `32` — the
  sub-country *region* bit — which both broke counting and contradicted
  the disclosure. Re-verify the constants against the source on every
  upgrade; the entrypoint comment lists them.
