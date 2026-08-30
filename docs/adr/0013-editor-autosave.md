# ADR-0013: Editor autosave, encrypted and on-device

- **Status**: Accepted
- **Date**: 2026-08-30

## Context

Refreshing the editor threw away the working document: you were back at
"Choose a PDF to edit" with every applied operation gone. That was a
deliberate position, written into the code — *"deliberately NOT
persisted: uploads don't survive a refresh either, and the privacy
promise is 'leave nothing behind'"* — not an oversight.

It was also the single most-felt papercut in the product, and it is not
a consequence of being client-side. Browsers have had IndexedDB and OPFS
for years; a PDF plus its edit history fits comfortably. Nothing about
saving locally requires a server, and the "cloud editing" idea it might
otherwise justify is a much bigger, much less necessary hammer.

The real objection was never technical, it was the **shared computer**:
today, closing the tab leaves nothing for the next person to find.

## Decision

Autosave the editor's working document to IndexedDB, sealed, with the
key kept separately.

1. **Sealed before it is stored.** `pz_crypto::seal` (AES-256-GCM) with
   a fresh 32-byte random key, so what sits on disk is ciphertext. The
   app gained a `pz-crypto` dependency for this; it was already in the
   graph via the engine.
2. **The key lives apart from the bytes** — localStorage, while the
   ciphertext is in IndexedDB. "Discard" removes the key *first*, so the
   document is unreadable the instant it is dismissed, even if the
   browser is lazy about reclaiming the blob. Deleting 32 bytes is far
   more reliable than trying to overwrite 50 MB (crypto-shredding).
3. **Restored only when asked.** Returning to the editor shows an offer
   naming the file and roughly when it was saved, with Restore and
   Discard. Silently reloading someone's document would be exactly the
   shared-computer failure the old position was protecting against.
4. **It expires.** Anything older than 24 hours is dropped unread on the
   next visit.
5. **It never interrupts editing.** Every storage failure (private
   browsing, quota) is swallowed; autosave is a safety net, not a
   feature the editor depends on.

## Scope, stated honestly

This saves the **working document** — every operation that has been
applied. Ink or text still floating on the canvas since the last bake is
not included. The editor bakes before any document operation, so the
uncovered window is small, but it is not zero, and the UI does not claim
otherwise.

## Consequences

- The refresh papercut is gone without a server, an account, or a
  subscription. Worth remembering when pricing a paid tier: surviving a
  refresh is free and local — what a paid tier could actually sell is
  *cross-device* sync, which is a different product.
- The privacy promise changes shape, so the Privacy page says it
  plainly: the editor keeps an encrypted copy on your device, it expires
  in a day, and Discard erases the key.
- `tests/ui/autosave.spec.js` drives the real bundle: store, reload,
  offer, restore, and a discard that provably leaves neither key nor
  record. It also asserts the stored bytes are **not** a raw PDF.
- Trap found while writing those tests: a probe that opens the IndexedDB
  database *without* an `onupgradeneeded` handler can win the race
  against the app, create an empty version-1 database, and permanently
  break autosave for that page — the app's later open sees the right
  version and never creates its store. Any future test touching this
  database must mirror the schema creation.
