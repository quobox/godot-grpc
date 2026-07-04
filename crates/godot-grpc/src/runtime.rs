//! Tokio runtime ownership and the `GrpcRuntime` engine singleton.
//!
//! The actual `tokio::runtime::Runtime` lives in a module-level static (not a
//! `#[var]` field) so it sidesteps the singleton's `init`-constructor
//! constraint and so its lifetime is bound to the extension stage lifecycle
//! (`InitStage::MainLoop`). **No Godot API is ever touched from a
//! Tokio worker thread** — only plain Rust data crosses back (see `bridge.rs`).

use std::sync::Mutex;
use std::time::Duration;

use godot::prelude::*;
use tokio::runtime::{Builder, Handle, Runtime};

/// Default Tokio worker-thread count.
const DEFAULT_WORKER_THREADS: usize = 2;

/// The background Tokio runtime. `None` until `start()`, taken+dropped on `shutdown()`.
static RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

/// Build and install the multi-threaded Tokio runtime. Idempotent: a second
/// call while already running is a no-op. Called from `on_stage_init(MainLoop)`.
pub(crate) fn start(worker_threads: usize) {
    let mut guard = RUNTIME.lock().expect("runtime mutex poisoned");
    if guard.is_some() {
        return;
    }
    let rt = Builder::new_multi_thread()
        .worker_threads(worker_threads.max(1))
        .thread_name("godot-grpc-tokio")
        .enable_all()
        .build()
        .expect("failed to build godot-grpc Tokio runtime");
    *guard = Some(rt);
    godot_print!("[godot-grpc] Tokio runtime started ({worker_threads} workers)");
}

/// Drop the runtime, giving in-flight tasks a bounded window to wind down.
/// Called from `on_stage_deinit(MainLoop)`.
pub(crate) fn shutdown() {
    if let Some(rt) = RUNTIME.lock().expect("runtime mutex poisoned").take() {
        rt.shutdown_timeout(Duration::from_secs(2));
        godot_print!("[godot-grpc] Tokio runtime shut down");
    }
}

/// A clonable handle for spawning work onto the runtime from the main thread.
/// `None` if the runtime is not currently running.
pub(crate) fn handle() -> Option<Handle> {
    RUNTIME
        .lock()
        .expect("runtime mutex poisoned")
        .as_ref()
        .map(|rt| rt.handle().clone())
}

/// GDScript-facing handle to the runtime: configuration and introspection.
/// The runtime itself is owned by the module static above.
#[derive(GodotClass)]
#[class(init, base = Object, singleton)]
pub struct GrpcRuntime {
    base: Base<Object>,
}

#[godot_api]
impl GrpcRuntime {
    /// Whether the background Tokio runtime is currently running.
    #[func]
    fn is_running(&self) -> bool {
        handle().is_some()
    }

    /// The default worker-thread count the runtime is started with.
    #[func]
    fn default_worker_threads(&self) -> i64 {
        DEFAULT_WORKER_THREADS as i64
    }
}
