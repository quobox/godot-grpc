//! `GrpcCall`: a handle to one in-flight RPC, exposing the result as signals.
//!
//! Tier 1 emits raw `PackedByteArray`s; tier 2 carries the method's input/output
//! `MessageDescriptor`s so `send_dict` encodes request dicts and `deliver`
//! decodes responses into `GrpcMessage`s. The `completed`/`stream_item` signals
//! therefore carry `Variant` (a `PackedByteArray` for tier 1, a `GrpcMessage`
//! for tier 2). All decoding happens here on the Godot main thread.

use godot::prelude::*;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;

#[cfg(feature = "tier2")]
use prost::Message;
#[cfg(feature = "tier2")]
use prost_reflect::{DynamicMessage, MessageDescriptor};

use crate::bridge::PumpEvent;
use crate::error::GrpcStatus;
#[cfg(feature = "tier2")]
use crate::{convert, message::GrpcMessage};

/// Optional message descriptor passed into call setup. Tier 2 carries a real
/// descriptor; with tier 2 off it degrades to `()` so `channel.rs` / `call.rs`
/// compile without `prost-reflect`.
#[cfg(feature = "tier2")]
pub(crate) type OptMsgDesc = Option<MessageDescriptor>;
#[cfg(not(feature = "tier2"))]
pub(crate) type OptMsgDesc = ();

#[derive(GodotClass)]
#[class(no_init, base = RefCounted)]
pub struct GrpcCall {
    /// Bridge-registry id, used to deregister on cancel.
    id: u64,
    /// Aborts the in-flight Tokio task on `cancel()`.
    abort: Option<AbortHandle>,
    /// Set once a terminal signal (completed/failed/cancelled) has fired, so
    /// `finished` fires exactly once even under cancel-vs-complete races.
    terminal: bool,
    /// Outbound sink for client-/bidi-streaming requests; `None` otherwise.
    outbound: Option<UnboundedSender<Vec<u8>>>,
    /// Tier-2 request type, for `send_dict`.
    #[cfg(feature = "tier2")]
    input: Option<MessageDescriptor>,
    /// Tier-2 response type, for decoding into `GrpcMessage`.
    #[cfg(feature = "tier2")]
    output: Option<MessageDescriptor>,
    base: Base<RefCounted>,
}

#[godot_api]
impl GrpcCall {
    /// One server-/bidi-stream message: `PackedByteArray` (tier 1) or `GrpcMessage` (tier 2).
    #[signal]
    fn stream_item(message: Variant);

    /// Success/stream-end: `PackedByteArray` (tier 1) or `GrpcMessage` (tier 2).
    #[signal]
    fn completed(response: Variant);

    /// Failure with the gRPC/transport status.
    #[signal]
    fn failed(status: Gd<GrpcStatus>);

    /// Emitted if the call is cancelled before completing.
    #[signal]
    fn cancelled();

    /// Unified terminal signal, convenient for `await`: carries the response on
    /// success, or `null` on failure/cancellation. Fires exactly once, together
    /// with the matching `completed` / `failed` / `cancelled` signal. Generated
    /// unary methods `await` this so a failed RPC resumes (returning null)
    /// instead of hanging.
    #[signal]
    fn finished(response: Variant);

    /// Send a raw encoded request message (tier-1 client-/bidi-streaming).
    #[func]
    fn send(&self, message: PackedByteArray) {
        self.send_raw(message.to_vec());
    }

    /// Send a request message as a `Dictionary` (tier-2 client-/bidi-streaming),
    /// encoded via the method's input descriptor.
    #[cfg(feature = "tier2")]
    #[func]
    fn send_dict(&self, message: VarDictionary) {
        let Some(input) = self.input.as_ref() else {
            godot_error!("[godot-grpc] send_dict() on a call without a tier-2 input type");
            return;
        };
        match convert::dict_to_message(input.clone(), &message) {
            Ok(dm) => self.send_raw(dm.encode_to_vec()),
            Err(e) => godot_error!("[godot-grpc] send_dict: {e}"),
        }
    }

    /// Close the outbound stream (client-/bidi-streaming): no more requests.
    #[func]
    fn close_send(&mut self) {
        self.outbound = None;
    }

    /// Cancel an in-flight call: aborts the underlying task and emits
    /// `cancelled()` (and the unified `finished(null)`). Safe to call after the
    /// call already finished (it's then a no-op beyond the signals).
    #[func]
    fn cancel(&mut self) {
        if let Some(abort) = self.abort.take() {
            abort.abort();
        }
        crate::bridge::deregister(self.id);
        self.finish_cancelled();
    }
}

impl GrpcCall {
    /// Create a call handle, optionally carrying tier-2 input/output descriptors.
    pub(crate) fn create(id: u64, input: OptMsgDesc, output: OptMsgDesc) -> Gd<Self> {
        #[cfg(not(feature = "tier2"))]
        let _ = (input, output);
        Gd::from_init_fn(|base| GrpcCall {
            id,
            abort: None,
            terminal: false,
            outbound: None,
            #[cfg(feature = "tier2")]
            input,
            #[cfg(feature = "tier2")]
            output,
            base,
        })
    }

    /// Attach the in-flight task's abort handle (for `cancel()`).
    pub(crate) fn set_abort(&mut self, abort: AbortHandle) {
        self.abort = Some(abort);
    }

    /// Attach the outbound message sink (client-/bidi-streaming calls).
    pub(crate) fn set_outbound(&mut self, tx: UnboundedSender<Vec<u8>>) {
        self.outbound = Some(tx);
    }

    fn send_raw(&self, bytes: Vec<u8>) {
        match &self.outbound {
            Some(tx) if tx.send(bytes).is_ok() => {}
            Some(_) => godot_error!("[godot-grpc] send() after the stream closed"),
            None => godot_error!("[godot-grpc] send() on a non-client-streaming call"),
        }
    }

    /// Emit the user-facing signal for a terminal/stream event. Main thread only
    /// (called from `bridge::drain`).
    pub(crate) fn deliver(&mut self, event: PumpEvent) {
        match event {
            PumpEvent::StreamItem { bytes, .. } => match self.payload(bytes) {
                // Non-terminal; suppressed once a terminal signal has fired.
                Ok(v) if !self.terminal => {
                    self.signals().stream_item().emit(&v);
                }
                Ok(_) => {}
                Err(status) => self.finish_failed(&status),
            },
            PumpEvent::Completed { bytes, .. } => match self.payload(bytes) {
                Ok(v) => self.finish_completed(&v),
                Err(status) => self.finish_failed(&status),
            },
            PumpEvent::Failed { code, message, .. } => {
                let status = GrpcStatus::create(code, &message);
                self.finish_failed(&status);
            }
            PumpEvent::Cancelled { .. } => self.finish_cancelled(),
        }
    }

    // Terminal transitions funnel through these guarded helpers so the
    // `finished` signal fires exactly once (spec'd contract), even if a terminal
    // event and a `cancel()` race. `stream_item` is the only non-terminal signal.

    fn finish_completed(&mut self, response: &Variant) {
        if self.terminal {
            return;
        }
        self.terminal = true;
        self.signals().completed().emit(response);
        self.signals().finished().emit(response);
    }

    fn finish_failed(&mut self, status: &Gd<GrpcStatus>) {
        if self.terminal {
            return;
        }
        self.terminal = true;
        self.signals().failed().emit(status);
        self.signals().finished().emit(&Variant::nil());
    }

    fn finish_cancelled(&mut self) {
        if self.terminal {
            return;
        }
        self.terminal = true;
        self.signals().cancelled().emit();
        self.signals().finished().emit(&Variant::nil());
    }

    /// Turn response bytes into a signal payload: tier 2 decodes to a
    /// `GrpcMessage`; tier 1 returns the raw `PackedByteArray`.
    fn payload(&self, bytes: Vec<u8>) -> Result<Variant, Gd<GrpcStatus>> {
        #[cfg(feature = "tier2")]
        if let Some(desc) = &self.output {
            return match DynamicMessage::decode(desc.clone(), bytes.as_slice()) {
                Ok(dm) => Ok(GrpcMessage::from_dynamic(dm).to_variant()),
                Err(e) => Err(GrpcStatus::create(
                    i32::from(tonic::Code::Internal) as i64,
                    &format!("failed to decode response: {e}"),
                )),
            };
        }
        Ok(PackedByteArray::from(bytes.as_slice()).to_variant())
    }
}
