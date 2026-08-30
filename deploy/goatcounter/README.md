# GoatCounter sidecar — built, not wired

This directory holds a working, self-hosted [GoatCounter](https://www.goatcounter.com/)
sidecar: a pinned binary, an entrypoint that forces its collection
settings to "page path + country, nothing else", and the two nginx
snippets that used to front it.

**Nothing in the shipped app talks to it.** Page counting was built,
deployed and verified, then deliberately removed — see
[ADR-0012](../../docs/adr/0012-anonymous-page-counting.md) for the full
reasoning. The app makes no requests of its own; the Privacy page says
so, and that claim is meant to stay true.

It is kept here because a future paid tier (cloud save, cross-device
editing) would need server-side infrastructure anyway, and this is a
known-good starting point.

## Re-wiring it (if that day comes)

1. `COPY deploy/goatcounter/gc-stub.conf /etc/nginx/pz-gc.conf` in the
   root `Dockerfile`.
2. Restore the `location = /gc/count` block in `deploy/nginx.conf`.
3. Add the `goatcounter` service back to `docker-compose.yml` and mount
   `gc-proxy.conf` over the stub.
4. Restore the beacon module and its tests from git history
   (commit `60bf108`), and restore the disclosure on the Privacy page
   **before** any of the above ships.

## Traps worth not rediscovering

- `collect` is an **iota bitmask** in GoatCounter's `settings.go`. In
  v2.7.0: Referrer=2, UserAgent=4, ScreenSize=8, **Location=16**,
  LocationRegion=32, Language=64, Session=128. Shipping `32` (region)
  instead of `16` (country) silently collects the wrong thing.
- `proxy_pass http://goatcounter:8080;` without the `/count` URI makes
  every beacon 404 — counting silently never works.
- nginx resolves the upstream hostname once at startup; recreate the
  sidecar alone and beacons 504 until the web container restarts.
- GoatCounter logs the raw User-Agent of bot-classified requests; the
  entrypoint wipes that table on boot.
