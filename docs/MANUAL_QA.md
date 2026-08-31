# Manual QA — the things headless can't check

Almost everything here is covered by `./scripts/ui-test.sh` (Playwright
against the real wasm bundle). This file lists what genuinely needs a
human with a real browser, why automation can't reach it, and exactly
what to click.

Run these against a **built bundle**, not `dx serve` — the PWA files,
`/ffmpeg/` and `/ocr/` are copied to the site root by
`scripts/build-web.sh` and 404 in dev:

```sh
./scripts/build-web.sh
podman run -d --name privzapp-web -p 127.0.0.1:8090:80 privzapp-web:latest
# or: docker compose up -d --build
```

Tick the sections a change actually touches. Note the browser and OS you
used; several of these differ between Chromium, Firefox and Safari.

---

## 1. Folder drag-and-drop  *(mostly automated now)*

**This used to be a manual section.** It was written on the belief that a
real directory drop couldn't be scripted — `webkitGetAsEntry()` does
return `null` for programmatically built `DataTransferItem`s. That is
true of the standard API, but Chrome DevTools Protocol's
`Input.dispatchDragEvent` accepts real filesystem paths and Chrome builds
a genuine entry tree from them. `tests/ui/dropdir.spec.js` now drops real
folders from disk and covers relative paths, the >100-file `readEntries`
batching, OS-clutter skipping, structure preserved in the archive, plain
and mixed drops, and single-file tools staying inert.

What is still worth a human, because CDP drives Chromium only:

- [ ] **Firefox and Safari.** Drag a folder onto `/tool/zip-files/` in
      each. Directory-entry support differs between engines; note
      anything that misbehaves.
- [ ] **A real file manager, once.** CDP synthesizes the drop faithfully
      enough for the entry API, but dragging from Finder/Explorer/Nautilus
      is the actual user gesture — worth one confirmation per platform.

## 2. Editor autosave  *(touched: `app/assets/autosave.js`, `app/src/autosave.rs`, `editor.rs` — ADR-0013)*

Covered by `tests/ui/autosave.spec.js`; what needs eyes is how it *feels*
and how it behaves with a real, large document.

- [ ] **Refresh mid-edit.** Open a PDF in `/tool/edit-pdf/`, apply a
      couple of operations (rotate, page numbers), then press F5. You are
      offered the document back by name with a rough age. Click
      **Restore** — pages come back with the operations applied.
- [ ] **Discard.** Refresh again, click **Discard**. The offer goes away,
      and refreshing once more does not bring it back.
- [ ] **A big file.** Try a 20–50 MB PDF. Saving must not make the editor
      stutter; note if it does.
- [ ] **Private/incognito window.** Storage may be restricted. Editing
      must keep working normally — autosave simply doesn't happen. No
      error banner.
- [ ] **Expiry (optional, slow).** A save older than 24 h is dropped
      unread. To check without waiting, edit the record's `savedAt` in
      devtools → Application → IndexedDB → `pz-editor`.

## 2b. Indonesian copy — native review  *(ADR-0014)*

**Why not automated:** no test can judge whether a translation reads
naturally, and Google's guidance is explicitly against publishing
unreviewed machine-translated content at scale. The structure (routing,
`hreflang`, sitemap) is test-covered; the prose is not.

- [ ] **Read the `/id/` pages as a native speaker would.** Start with
      the highest-traffic tools: `/id/`, `/id/tool/merge-pdf/`,
      `/id/tool/compress-pdf/`, `/id/tool/convert-img/`.
- [ ] **Check the search-facing copy first** — the `<title>` and meta
      description are what people see before they click. Awkward
      phrasing there costs more than anywhere else.
- [ ] **Confirm format tokens stayed English** (PDF, JPG, ZIP, OCR,
      AES-256): those are what people actually type into a search box.
- [ ] Fix anything that reads like a translation in
      `crates/pz-core/src/i18n_id.rs` and `i18n_seo_id.rs`.
- [ ] **Do this before submitting the `/id/` URLs to Search Console.**

## 3. Service worker / offline  *(touched: `app/pwa/sw.js`, `scripts/build-web.sh`)*

**Why not automated:** the test server and the Playwright harness don't
exercise install/activate/update cycles realistically.

- [ ] **Install the PWA** (address-bar install icon). It opens in its own
      window with the right icon and name.
- [ ] **Offline.** Load the site, then turn off networking (or devtools →
      Network → Offline) and reload. The app still loads and tools still
      run.
- [ ] **Update flow.** Deploy a new build, reload twice. The new version
      appears rather than a stale cached shell.
- [ ] **Video/OCR offline.** Use a video tool and an OCR tool once while
      online (this downloads the ffmpeg / tesseract runtimes), then go
      offline and use them again — they should still work from cache.

## 4. Editor canvas interactions  *(touched: `app/assets/editor.js`, `editor.rs`)*

Playwright covers a lot of this (`tests/ui/editor.spec.js`), but pointer
feel, stylus and touch are not reproducible headless.

- [ ] **Draw with a real pointer** — mouse, then a touchscreen/stylus if
      available. Ink follows the cursor with no offset at 100 % zoom and
      while zoomed in.
- [ ] **Text boxes.** Place, type, drag, and edit an existing PDF text
      run. Fonts should not visibly change after baking.
- [ ] **Export.** Download the edited PDF and open it in a real PDF
      viewer (not just the browser) — Acrobat and Preview are the ones
      that complain loudest about malformed output.

## 5. Downloads and saving  *(touched: `app/src/save.rs`)*

- [ ] **Web download.** File lands in the browser's download folder with
      the expected name.
- [ ] **Mobile browser.** Download on Android/iOS — filename and the
      "open with" flow behave sensibly.

## 6. Android app  *(touched: `scripts/build-android.sh`, `save.rs`)*

Known broken, tracked in `docs/ROADMAP.md`:

- [ ] **File picker.** Tapping a dropzone opens the system picker. (It
      currently does nothing — the WebView needs
      `WebChromeClient.onShowFileChooser` wired up.)
- [ ] **Saving.** Files land somewhere the user can reach, not the
      app-private temp dir.
