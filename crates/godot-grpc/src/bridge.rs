//! Tokio → Godot main-thread bridge.
//!
//! A Tokio task produces a [`PumpEvent`] carrying **only plain Rust data** —
//! never a `Gd<T>` — and sends it over a process-wide `crossbeam-channel`. The
//! main thread drains the channel once per frame from
//! [`ExtensionLibrary::on_main_loop_frame`][crate::GodotGrpc] and dispatches
//! each event to its `GrpcCall`, which emits the user-facing signal.
//!
//! **Why a global per-frame drain (not a per-scene-tree pump Node):** gdext
//! 0.5's `on_main_loop_frame` hook
//! (Godot 4.5+) is a global main-thread per-frame callback that fits our single
//! global channel exactly, and avoids injecting/cleaning up a Node in every
//! scene tree. The drain logic is a plain function so it is also callable
//! directly from headless Rust tests.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crossbeam_channel::{Receiver, Sender, unbounded};
use godot::prelude::*;

use crate::call::GrpcCall;

/// An event from a Tokio task back to the main thread, carrying only plain Rust
/// data (never `Gd<T>`). `StreamItem` is non-terminal; the others end the call.
pub(crate) enum PumpEvent {
    /// A server-/bidi-stream message (non-terminal): one encoded message.
    StreamItem { call_id: u64, bytes: Vec<u8> },
    /// Success/end: the encoded response (unary) or empty bytes (stream end).
    Completed { call_id: u64, bytes: Vec<u8> },
    /// Failure: a gRPC/transport status code + message.
    Failed {
        call_id: u64,
        code: i64,
        message: String,
    },
    /// A cancellation event. Currently unused: `GrpcCall::cancel()` emits the
    /// `cancelled` signal directly on the main thread instead of via the channel.
    #[allow(dead_code)]
    Cancelled { call_id: u64 },
}

impl PumpEvent {
    fn call_id(&self) -> u64 {
        match self {
            PumpEvent::StreamItem { call_id, .. }
            | PumpEvent::Completed { call_id, .. }
            | PumpEvent::Failed { call_id, .. }
            | PumpEvent::Cancelled { call_id, .. } => *call_id,
        }
    }

    /// Whether this event ends the call (the call is then deregistered).
    fn is_terminal(&self) -> bool {
        !matches!(self, PumpEvent::StreamItem { .. })
    }
}

/// Process-wide event channel. The `Sender` is cloned into Tokio tasks; the
/// `Receiver` is drained only on the Godot main thread.
static CHANNEL: OnceLock<(Sender<PumpEvent>, Receiver<PumpEvent>)> = OnceLock::new();

fn channel() -> &'static (Sender<PumpEvent>, Receiver<PumpEvent>) {
    CHANNEL.get_or_init(unbounded)
}

/// A `Sender` clone for posting events back to the main thread from a Tokio task.
pub(crate) fn sender() -> Sender<PumpEvent> {
    channel().0.clone()
}

/// Allocate a unique id for a new in-flight call.
pub(crate) fn next_call_id() -> u64 {
    static NEXT_CALL_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_CALL_ID.fetch_add(1, Ordering::Relaxed)
}

thread_local! {
    /// Main-thread registry of in-flight calls, keyed by call id. Holds `Gd<GrpcCall>`
    /// (not `Send`), which is why it lives in a `thread_local`, never crossing threads.
    static REGISTRY: RefCell<HashMap<u64, Gd<GrpcCall>>> = RefCell::new(HashMap::new());
}

/// Register an in-flight call so the drain loop can route its events. Main thread only.
pub(crate) fn register(id: u64, call: Gd<GrpcCall>) {
    REGISTRY.with_borrow_mut(|r| {
        r.insert(id, call);
    });
}

fn take(id: u64) -> Option<Gd<GrpcCall>> {
    REGISTRY.with_borrow_mut(|r| r.remove(&id))
}

/// Drop a call from the registry without delivering (used by `GrpcCall::cancel`).
pub(crate) fn deregister(id: u64) {
    REGISTRY.with_borrow_mut(|r| {
        r.remove(&id);
    });
}

/// Drain all pending events and dispatch them to their calls. **Must run on the
/// Godot main thread** — invoked from `on_main_loop_frame`, or directly in tests.
pub(crate) fn drain() {
    let rx = &channel().1;
    while let Ok(event) = rx.try_recv() {
        let id = event.call_id();
        // Resolve the target call and release the registry borrow *before*
        // emitting (deliver -> signal -> GDScript handler may call back into
        // the registry, e.g. start another RPC). Terminal events deregister.
        let call = if event.is_terminal() {
            take(id)
        } else {
            REGISTRY.with_borrow(|r| r.get(&id).cloned())
        };
        if let Some(mut call) = call {
            call.bind_mut().deliver(event);
        }
    }
}
