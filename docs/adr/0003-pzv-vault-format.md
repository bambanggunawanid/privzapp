# ADR-0003: `.pzv` password-vault format instead of PDF-native encryption

- **Status**: Accepted
- **Date**: 2026-08-24

## Context

The roadmap called for password-protecting PDFs. PDF's standard security
handlers are weak-to-mediocre (RC4 legacy; AES-128/256 variants with
well-known cracking tooling for low-iteration KDFs), and `lopdf` 0.44 can
read but not write encrypted documents. Meanwhile users also want to
protect files that aren't PDFs.

## Decision

Ship a format-agnostic vault instead: **any** file can be sealed to a
`.pzv` file and opened again on any PrivZapp platform.

Layout: `"PZV1" (4) || salt (16) || nonce (12) || AES-256-GCM ciphertext+tag`.

- Key: PBKDF2-HMAC-SHA256, 600,000 rounds (OWASP 2023), random per-file
  salt. Pure Rust (`pbkdf2` crate, hmac-only features), wasm-safe.
- AEAD: AES-256-GCM via the existing `pz_crypto::seal`/`open`, fresh
  random nonce per seal.
- The magic byte prefix gives honest error messages ("not a vault" vs
  "wrong password") without weakening anything.

## Consequences

- Encrypted output only opens in PrivZapp (any platform), not in PDF
  readers. This is stated in the tool tagline; in exchange the crypto is
  uniformly strong instead of PDF-reader-compatible.
- Losing the password loses the file — GCM authentication makes partial
  recovery impossible. UI copy warns about this next to the password box.
- ~1s of deliberate key-derivation work per operation on slow devices;
  acceptable for an explicit "encrypt this" action.
- Version byte in the magic (`PZV1`) leaves room to migrate KDF/cipher
  without breaking old vaults.
