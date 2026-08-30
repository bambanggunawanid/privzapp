//! Engine dispatch. On the web the wasm engine runs inside a dedicated
//! Web Worker (ADR-0004) so a 200 MB zip can't freeze the tab; native
//! builds — and any browser where the worker fails to boot — run inline
//! exactly as before.
//!
//! The worker is the SAME dx-built module loaded a second time: a tiny
//! blob script imports the app's entry JS (auto-initializing), and
//! `main()` detects the worker context (no `Window`) and registers the
//! engine message handler instead of launching the UI. File bytes cross
//! as transferable ArrayBuffers, so peak memory stays ~1x.

use pz_core::{InputFile, OutputFile, ToolOptions};

/// Run a tool. Async so the web build can await the worker round-trip.
/// Errors are display strings ready for the UI.
pub async fn run(
    slug: &'static str,
    files: Vec<InputFile>,
    opts: &ToolOptions,
) -> Result<Vec<OutputFile>, String> {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        web::run(slug, files, opts).await
    }
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        pz_engine::run(slug, &files, opts).map_err(|e| e.to_string())
    }
}

/// Entry hook for `main()`: returns true when this wasm instance is the
/// engine worker (no `Window`), in which case the UI must not launch.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub fn maybe_worker_main() -> bool {
    web::maybe_worker_main()
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod web {
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

    use futures_channel::oneshot;
    use js_sys::{Array, Reflect, Uint8Array};
    use pz_core::{InputFile, OutputFile, ToolOptions};
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsValue};
    use web_sys::{DedicatedWorkerGlobalScope, MessageEvent, Worker, WorkerOptions, WorkerType};

    /// serde mirror of ToolOptions so the engine crates stay serde-free.
    #[derive(serde::Serialize, serde::Deserialize, Default)]
    struct WireOpts {
        quality: u8,
        width: u32,
        height: u32,
        format: String,
        pages: String,
        angle: i32,
        text: String,
        x: u32,
        y: u32,
        password: String,
        scale: u32,
        percent: u32,
    }

    impl From<&ToolOptions> for WireOpts {
        fn from(o: &ToolOptions) -> Self {
            Self {
                quality: o.quality,
                width: o.width,
                height: o.height,
                format: o.format.clone(),
                pages: o.pages.clone(),
                angle: o.angle,
                text: o.text.clone(),
                x: o.x,
                y: o.y,
                password: o.password.clone(),
                scale: o.scale,
                percent: o.percent,
            }
        }
    }

    impl From<WireOpts> for ToolOptions {
        fn from(w: WireOpts) -> Self {
            Self {
                quality: w.quality,
                width: w.width,
                height: w.height,
                format: w.format,
                pages: w.pages,
                angle: w.angle,
                text: w.text,
                x: w.x,
                y: w.y,
                password: w.password,
                scale: w.scale,
                percent: w.percent,
                ..Self::default()
            }
        }
    }

    struct Job {
        id: u32,
        slug: &'static str,
        files: Vec<InputFile>,
        opts: ToolOptions,
    }

    enum State {
        Untried,
        /// Worker created, waiting for its "pz-ready" handshake.
        Booting(Vec<Job>),
        Ready,
        /// Too many failures — run inline for the rest of this page load.
        Broken,
    }

    type Sender = oneshot::Sender<Result<Vec<OutputFile>, String>>;

    thread_local! {
        static STATE: RefCell<State> = const { RefCell::new(State::Untried) };
        static WORKER: RefCell<Option<Worker>> = const { RefCell::new(None) };
        static PENDING: RefCell<HashMap<u32, Sender>> = RefCell::new(HashMap::new());
        static NEXT_ID: Cell<u32> = const { Cell::new(1) };
        static FAILURES: Cell<u32> = const { Cell::new(0) };
        // Event closures must outlive their registration.
        static KEEP: RefCell<Vec<Closure<dyn FnMut(MessageEvent)>>> = RefCell::new(Vec::new());
    }

    fn run_inline(job: &Job) -> Result<Vec<OutputFile>, String> {
        pz_engine::run(job.slug, &job.files, &job.opts).map_err(|e| e.to_string())
    }

    /// Publish the active mode where the UI tests can assert on it.
    fn set_mode(mode: &str) {
        if let Some(win) = web_sys::window() {
            let _ = Reflect::set(&win, &"pzEngineMode".into(), &mode.into());
        }
    }

    pub async fn run(
        slug: &'static str,
        files: Vec<InputFile>,
        opts: &ToolOptions,
    ) -> Result<Vec<OutputFile>, String> {
        let job = Job {
            id: NEXT_ID.with(|n| {
                let id = n.get();
                n.set(id + 1);
                id
            }),
            slug,
            files,
            opts: opts.clone(),
        };

        // Broken (or spawn failure below) → inline, same as before workers.
        if STATE.with(|s| matches!(*s.borrow(), State::Broken)) {
            return run_inline(&job);
        }
        if STATE.with(|s| matches!(*s.borrow(), State::Untried)) {
            match spawn_worker() {
                Some(worker) => {
                    WORKER.with(|w| *w.borrow_mut() = Some(worker));
                    STATE.with(|s| *s.borrow_mut() = State::Booting(Vec::new()));
                    arm_boot_timeout();
                }
                None => {
                    STATE.with(|s| *s.borrow_mut() = State::Broken);
                    set_mode("inline");
                    return run_inline(&job);
                }
            }
        }

        let (tx, rx) = oneshot::channel();
        PENDING.with(|p| p.borrow_mut().insert(job.id, tx));
        if STATE.with(|s| matches!(*s.borrow(), State::Ready)) {
            send_job(&job);
        } else {
            // Still booting (Untried/Broken were resolved above; nothing
            // can change state between these two synchronous accesses).
            STATE.with(|s| {
                if let State::Booting(queue) = &mut *s.borrow_mut() {
                    queue.push(job);
                }
            });
        }
        match rx.await {
            Ok(result) => result,
            Err(_) => Err("engine call was cancelled".into()),
        }
    }

    /// Create the worker from the /pz-worker.js shim (written at build
    /// time by seo-gen) — a module that imports the app's own hashed
    /// entry. The entry auto-initializes the wasm and calls main(),
    /// which lands in maybe_worker_main() below. It must be a real
    /// same-origin URL, not a blob: — the glue fetches its .wasm by
    /// relative path and blob: can't be a base URL. In dev (`dx serve`)
    /// the shim doesn't exist; the 404 fires the error handler and
    /// everything runs inline as before.
    fn spawn_worker() -> Option<Worker> {
        let opts = WorkerOptions::new();
        opts.set_type(WorkerType::Module);
        let worker = Worker::new_with_options("/pz-worker.js", &opts).ok()?;

        let on_message = Closure::wrap(Box::new(client_on_message) as Box<dyn FnMut(MessageEvent)>);
        worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        KEEP.with(|k| k.borrow_mut().push(on_message));
        // A failed import / wasm fetch fires the worker's error event.
        let on_error = Closure::wrap(
            Box::new(move |_e: MessageEvent| worker_failed()) as Box<dyn FnMut(MessageEvent)>
        );
        worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        KEEP.with(|k| k.borrow_mut().push(on_error));
        Some(worker)
    }

    /// Boot watchdog: init failures inside the worker reject a promise
    /// (no error event reaches us), so a worker still Booting after 20 s
    /// is treated as failed and queued jobs run inline.
    fn arm_boot_timeout() {
        let Some(win) = web_sys::window() else { return };
        let cb = Closure::wrap(Box::new(|| {
            if STATE.with(|s| matches!(*s.borrow(), State::Booting(_))) {
                worker_failed();
            }
        }) as Box<dyn FnMut()>);
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            20_000,
        );
        cb.forget();
    }

    fn worker_failed() {
        WORKER.with(|w| {
            if let Some(worker) = w.borrow_mut().take() {
                worker.terminate();
            }
        });
        let queued = STATE.with(|s| {
            let mut state = s.borrow_mut();
            let queued = match &mut *state {
                State::Booting(q) => std::mem::take(q),
                _ => Vec::new(),
            };
            // A crashed worker respawns on the next call (isolation is the
            // point: a hostile file kills the worker, not the tab) — but a
            // repeat offender downgrades to inline for this page load.
            FAILURES.set(FAILURES.get() + 1);
            *state = if FAILURES.get() >= 2 {
                set_mode("inline");
                State::Broken
            } else {
                State::Untried
            };
            queued
        });
        // Jobs queued during boot still have callers awaiting them.
        for job in &queued {
            let result = run_inline(job);
            if let Some(tx) = PENDING.with(|p| p.borrow_mut().remove(&job.id)) {
                let _ = tx.send(result);
            }
        }
        // In-flight jobs were transferred to the dead worker; their input
        // buffers are gone, so all we can do is report the failure.
        let stranded: Vec<Sender> =
            PENDING.with(|p| p.borrow_mut().drain().map(|(_, tx)| tx).collect());
        for tx in stranded {
            let _ = tx.send(Err(
                "the engine crashed on this file — running the next attempt in-page".into(),
            ));
        }
    }

    fn client_on_message(evt: MessageEvent) {
        let data = evt.data();
        if let Some(panic) = data.as_string().filter(|s| s.starts_with("pz-panic:")) {
            web_sys::console::error_1(&panic.as_str().into());
            worker_failed();
            return;
        }
        if data.as_string().as_deref() == Some("pz-ready") {
            FAILURES.set(0);
            set_mode("worker");
            let queued = STATE.with(|s| {
                let mut state = s.borrow_mut();
                let queued = match &mut *state {
                    State::Booting(q) => std::mem::take(q),
                    _ => Vec::new(),
                };
                *state = State::Ready;
                queued
            });
            for job in &queued {
                send_job(job);
            }
            return;
        }
        let get = |key: &str| Reflect::get(&data, &key.into()).ok();
        let Some(id) = get("id").and_then(|v| v.as_f64()) else {
            return;
        };
        let Some(tx) = PENDING.with(|p| p.borrow_mut().remove(&(id as u32))) else {
            return;
        };
        let ok = get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let err = get("err")
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| "engine error".into());
            let _ = tx.send(Err(err));
            return;
        }
        let arr = |key: &str| {
            get(key)
                .and_then(|v| v.dyn_into::<Array>().ok())
                .unwrap_or_else(Array::new)
        };
        let (names, mimes, buffers) = (arr("names"), arr("mimes"), arr("buffers"));
        let mut outputs = Vec::with_capacity(names.length() as usize);
        for i in 0..names.length() {
            outputs.push(OutputFile {
                name: names.get(i).as_string().unwrap_or_default(),
                mime: intern_mime(mimes.get(i).as_string().unwrap_or_default()),
                bytes: Uint8Array::new(&buffers.get(i)).to_vec(),
            });
        }
        let _ = tx.send(Ok(outputs));
    }

    fn send_job(job: &Job) {
        let msg = js_sys::Object::new();
        let names = Array::new();
        let buffers = Array::new();
        let transfer = Array::new();
        for f in &job.files {
            names.push(&JsValue::from_str(&f.name));
            let bytes = Uint8Array::from(f.bytes.as_slice());
            let buf = bytes.buffer();
            buffers.push(&buf);
            transfer.push(&buf);
        }
        let opts_json = serde_json::to_string(&WireOpts::from(&job.opts)).unwrap_or_default();
        let _ = Reflect::set(&msg, &"id".into(), &JsValue::from_f64(job.id as f64));
        let _ = Reflect::set(&msg, &"slug".into(), &job.slug.into());
        let _ = Reflect::set(&msg, &"names".into(), &names);
        let _ = Reflect::set(&msg, &"buffers".into(), &buffers);
        let _ = Reflect::set(&msg, &"opts".into(), &opts_json.as_str().into());
        let sent = WORKER.with(|w| {
            w.borrow()
                .as_ref()
                .map(|worker| worker.post_message_with_transfer(&msg, &transfer).is_ok())
                .unwrap_or(false)
        });
        if !sent {
            // postMessage itself failed — the transfer never happened, so
            // the bytes are still valid: finish inline.
            let result = run_inline(job);
            if let Some(tx) = PENDING.with(|p| p.borrow_mut().remove(&job.id)) {
                let _ = tx.send(result);
            }
        }
    }

    /// OutputFile.mime is `&'static str`; worker responses arrive as owned
    /// strings. The engine emits ~a dozen distinct mimes, so interning
    /// with a leak-on-first-sight table is bounded.
    fn intern_mime(mime: String) -> &'static str {
        thread_local! {
            static MIMES: RefCell<HashMap<String, &'static str>> = RefCell::new(HashMap::new());
        }
        MIMES.with(|m| {
            let mut map = m.borrow_mut();
            if let Some(s) = map.get(&mime) {
                return *s;
            }
            let leaked: &'static str = Box::leak(mime.clone().into_boxed_str());
            map.insert(mime, leaked);
            leaked
        })
    }

    // ---- worker side ------------------------------------------------------

    pub fn maybe_worker_main() -> bool {
        if web_sys::window().is_some() {
            return false;
        }
        // A panicking worker aborts with a bare "unreachable" — forward
        // the actual panic text to the client so it can be surfaced.
        std::panic::set_hook(Box::new(|info| {
            let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
            let _ = scope.post_message(&JsValue::from_str(&format!("pz-panic:{info}")));
        }));
        let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
        let handler = Closure::wrap(Box::new(worker_on_message) as Box<dyn FnMut(MessageEvent)>);
        scope.set_onmessage(Some(handler.as_ref().unchecked_ref()));
        handler.forget();
        let _ = scope.post_message(&JsValue::from_str("pz-ready"));
        true
    }

    fn worker_on_message(evt: MessageEvent) {
        let data = evt.data();
        let get = |key: &str| Reflect::get(&data, &key.into()).ok();
        let id = get("id").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let slug = get("slug").and_then(|v| v.as_string()).unwrap_or_default();
        let arr = |key: &str| {
            get(key)
                .and_then(|v| v.dyn_into::<Array>().ok())
                .unwrap_or_else(Array::new)
        };
        let (names, buffers) = (arr("names"), arr("buffers"));
        let mut files = Vec::with_capacity(names.length() as usize);
        for i in 0..names.length() {
            files.push(InputFile {
                name: names.get(i).as_string().unwrap_or_default(),
                bytes: Uint8Array::new(&buffers.get(i)).to_vec(),
            });
        }
        let opts: ToolOptions = get("opts")
            .and_then(|v| v.as_string())
            .and_then(|json| serde_json::from_str::<WireOpts>(&json).ok())
            .map(Into::into)
            .unwrap_or_default();

        let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
        let reply = js_sys::Object::new();
        let _ = Reflect::set(&reply, &"id".into(), &JsValue::from_f64(id));
        match pz_engine::run(&slug, &files, &opts) {
            Ok(outputs) => {
                let names = Array::new();
                let mimes = Array::new();
                let buffers = Array::new();
                let transfer = Array::new();
                for o in &outputs {
                    names.push(&JsValue::from_str(&o.name));
                    mimes.push(&JsValue::from_str(o.mime));
                    let bytes = Uint8Array::from(o.bytes.as_slice());
                    let buf = bytes.buffer();
                    buffers.push(&buf);
                    transfer.push(&buf);
                }
                let _ = Reflect::set(&reply, &"ok".into(), &JsValue::TRUE);
                let _ = Reflect::set(&reply, &"names".into(), &names);
                let _ = Reflect::set(&reply, &"mimes".into(), &mimes);
                let _ = Reflect::set(&reply, &"buffers".into(), &buffers);
                let _ = scope.post_message_with_transfer(&reply, &transfer);
            }
            Err(e) => {
                let _ = Reflect::set(&reply, &"ok".into(), &JsValue::FALSE);
                let _ = Reflect::set(&reply, &"err".into(), &e.to_string().as_str().into());
                let _ = scope.post_message(&reply);
            }
        }
    }
}
