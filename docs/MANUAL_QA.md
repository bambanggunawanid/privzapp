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

## 1. Folder drag-and-drop  *(touched: `app/assets/dropdir.js`, `app/src/pages/tool.rs`, `pz-archive`)*

**Why not automated:** a real directory drop is the one thing Playwright
cannot synthesize. `webkitGetAsEntry()` returns `null` for
programmatically constructed `DataTransferItem`s, so the entry tree that
the whole feature walks only exists during a genuine OS drag. The specs
in `tests/ui/dropdir.spec.js` therefore enter through `pzIngestEntries`
— the exact function the drop listener calls — and cover everything
after that point. The gesture itself is what you are testing here.

Prepare a folder with subfolders, e.g.

```
holiday/
  notes.txt
  photos/            # put 120+ files in here for step 4
    img-001.jpg …
    raw/
      deep.txt
```

- [ ] **Drop a folder on Create ZIP.** Open `/tool/zip-files/`, drag
      `holiday/` from the file manager onto the dropzone. Every file
      appears in the list with its **relative path** (`photos/raw/deep.txt`,
      not just `deep.txt`).
- [ ] **The archive keeps the structure.** Run it, download the zip, open
      it. Subfolders are preserved, and same-named files in different
      subfolders both survive.
- [ ] **OS clutter is skipped.** If the folder has `.DS_Store`,
      `Thumbs.db` or `desktop.ini`, they are not in the list.
- [ ] **Large directory.** Drop a folder with **more than 100 files** —
      Chrome's `readEntries` returns at most 100 per call, so this proves
      the walker keeps looping. All files must appear, not just 100.
- [ ] **Plain file drops still work.** Select several loose files (no
      folder) and drop them. They load through the normal path.
- [ ] **Mixed drop.** Drop a folder *and* loose files together.
- [ ] **Single-file tools are untouched.** Drop a folder on
      `/tool/unzip/` (single-file) and on the editor. Nothing should be
      swallowed or silently queued — the drop behaves as it did before.
- [ ] **Second drop appends.** Drop one folder, then another; the file
      list grows rather than being replaced.
- [ ] **Firefox and Safari.** Repeat the first two steps. Directory entry
      support differs; note anything that misbehaves.

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
