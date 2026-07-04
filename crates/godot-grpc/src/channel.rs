//! `GrpcChannel`: a connection to one gRPC endpoint over TCP or a Unix Domain
//! Socket.
//!
//! Connection is **lazy**: construction is synchronous and the transport
//! connection is established on the first RPC, on the Tokio runtime. Connection
//! errors surface at call time via the `failed(GrpcStatus)` signal. The
//! underlying `tonic::transport::Channel` is cheap to clone and multiplexes
//! concurrent RPCs, so one `GrpcChannel` is shared across many calls.
//!
//! The `start_*` functions are shared by the tier-1 `#[func]`s here (which pass
//! no descriptors) and by tier-2 `GrpcServiceStub` (which passes the method's
//! input/output `MessageDescriptor`s for dict encoding / message decoding).

use crossbeam_channel::Sender;
use godot::prelude::*;
use tokio::sync::mpsc::unbounded_channel;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::Code;
use tonic::codegen::http::uri::PathAndQuery;
use tonic::transport::Channel;

use crate::bridge::{self, PumpEvent};
use crate::call::{GrpcCall, OptMsgDesc};
use crate::{runtime, transport};

#[derive(GodotClass)]
#[class(no_init, base = RefCounted)]
pub struct GrpcChannel {
    channel: Channel,
    base: Base<RefCounted>,
}

#[godot_api]
impl GrpcChannel {
    /// Connect (lazily) over TCP. `endpoint` like `"http://127.0.0.1:50051"`.
    /// Returns `null` and logs an error if the URI is malformed.
    #[func]
    fn tcp(endpoint: GString) -> Option<Gd<GrpcChannel>> {
        Self::build(&endpoint.to_string())
    }

    /// Connect (lazily) over a Unix Domain Socket path. Linux/macOS only.
    #[cfg(unix)]
    #[func]
    fn uds(path: GString) -> Option<Gd<GrpcChannel>> {
        Self::build(&format!("unix://{path}"))
    }

    /// Tier-1 unary call: `path` is `"/<package>.<Service>/<Method>"`, `request`
    /// the encoded request; the response arrives as `completed` (PackedByteArray).
    #[func]
    fn unary_call(&self, path: GString, request: PackedByteArray) -> Gd<GrpcCall> {
        start_unary(
            self.channel.clone(),
            &path.to_string(),
            request.to_vec(),
            Default::default(),
        )
    }

    /// Tier-1 server-streaming call. Responses arrive as `stream_item`, ending
    /// with `completed` (empty), or `failed`.
    #[func]
    fn server_stream_call(&self, path: GString, request: PackedByteArray) -> Gd<GrpcCall> {
        start_server_stream(
            self.channel.clone(),
            &path.to_string(),
            request.to_vec(),
            Default::default(),
        )
    }

    /// Tier-1 client-streaming call. Send with `GrpcCall.send()` / `close_send()`.
    #[func]
    fn client_stream_call(&self, path: GString) -> Gd<GrpcCall> {
        start_client_stream(
            self.channel.clone(),
            &path.to_string(),
            Default::default(),
            Default::default(),
        )
    }

    /// Tier-1 bidirectional call. Send with `GrpcCall.send()` / `close_send()`.
    #[func]
    fn bidi_call(&self, path: GString) -> Gd<GrpcCall> {
        start_bidi(
            self.channel.clone(),
            &path.to_string(),
            Default::default(),
            Default::default(),
        )
    }
}

impl GrpcChannel {
    fn build(uri: &str) -> Option<Gd<GrpcChannel>> {
        match transport::connect_lazy(uri) {
            Ok(channel) => Some(Gd::from_init_fn(|base| GrpcChannel { channel, base })),
            Err(e) => {
                godot_error!("[godot-grpc] invalid endpoint {uri:?}: {e}");
                None
            }
        }
    }

    /// Clone the underlying tonic channel (used by tier-2 `GrpcServiceStub`).
    #[cfg(feature = "tier2")]
    pub(crate) fn channel(&self) -> Channel {
        self.channel.clone()
    }
}

// ---------------------------------------------------------------------------
// Shared call starters (tier 1 passes `None` descriptors; tier 2 passes Some).
// ---------------------------------------------------------------------------

struct Begun {
    id: u64,
    call: Gd<GrpcCall>,
    sender: Sender<PumpEvent>,
    handle: tokio::runtime::Handle,
}

/// Allocate + register a call. On a stopped runtime, posts `Failed` and returns
/// the call via `Err` for the caller to return immediately.
fn begin(input: OptMsgDesc, output: OptMsgDesc) -> Result<Begun, Gd<GrpcCall>> {
    let id = bridge::next_call_id();
    let call = GrpcCall::create(id, input, output);
    bridge::register(id, call.clone());
    let sender = bridge::sender();
    match runtime::handle() {
        Some(handle) => Ok(Begun {
            id,
            call,
            sender,
            handle,
        }),
        None => {
            let _ = sender.send(PumpEvent::Failed {
                call_id: id,
                code: i32::from(Code::Unavailable) as i64,
                message: "godot-grpc Tokio runtime is not running".into(),
            });
            Err(call)
        }
    }
}

/// Parse the method path, posting `Failed` and returning the call on error.
fn parse_path(begun: &Begun, path: &str) -> Result<PathAndQuery, ()> {
    path.parse::<PathAndQuery>().map_err(|e| {
        let _ = begun.sender.send(PumpEvent::Failed {
            call_id: begun.id,
            code: i32::from(Code::InvalidArgument) as i64,
            message: format!("invalid method path {path:?}: {e}"),
        });
    })
}

/// Spawn the RPC `task` on the runtime and attach its abort handle to the call
/// (so `cancel()` can abort it). The shared tail of every `start_*`.
fn spawn_and_attach<F>(begun: Begun, task: impl FnOnce(u64, Sender<PumpEvent>) -> F) -> Gd<GrpcCall>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let Begun {
        id,
        mut call,
        sender,
        handle,
    } = begun;
    let join = handle.spawn(task(id, sender));
    call.bind_mut().set_abort(join.abort_handle());
    call
}

pub(crate) fn start_unary(
    channel: Channel,
    path: &str,
    request: Vec<u8>,
    output: OptMsgDesc,
) -> Gd<GrpcCall> {
    let begun = match begin(Default::default(), output) {
        Ok(b) => b,
        Err(call) => return call,
    };
    let path = match parse_path(&begun, path) {
        Ok(p) => p,
        Err(()) => return begun.call,
    };
    spawn_and_attach(begun, move |id, sender| async move {
        let event = match transport::unary(channel, path, request).await {
            Ok(bytes) => PumpEvent::Completed { call_id: id, bytes },
            Err(status) => fail(id, &status),
        };
        let _ = sender.send(event);
    })
}

pub(crate) fn start_server_stream(
    channel: Channel,
    path: &str,
    request: Vec<u8>,
    output: OptMsgDesc,
) -> Gd<GrpcCall> {
    let begun = match begin(Default::default(), output) {
        Ok(b) => b,
        Err(call) => return call,
    };
    let path = match parse_path(&begun, path) {
        Ok(p) => p,
        Err(()) => return begun.call,
    };
    spawn_and_attach(begun, move |id, sender| async move {
        match transport::open_server_stream(channel, path, request).await {
            Ok(stream) => pump_stream(id, &sender, stream).await,
            Err(status) => {
                let _ = sender.send(fail(id, &status));
            }
        }
    })
}

pub(crate) fn start_client_stream(
    channel: Channel,
    path: &str,
    input: OptMsgDesc,
    output: OptMsgDesc,
) -> Gd<GrpcCall> {
    let mut begun = match begin(input, output) {
        Ok(b) => b,
        Err(call) => return call,
    };
    let path = match parse_path(&begun, path) {
        Ok(p) => p,
        Err(()) => return begun.call,
    };
    let requests = attach_outbound(&mut begun.call);
    spawn_and_attach(begun, move |id, sender| async move {
        let event = match transport::client_streaming(channel, path, requests).await {
            Ok(bytes) => PumpEvent::Completed { call_id: id, bytes },
            Err(status) => fail(id, &status),
        };
        let _ = sender.send(event);
    })
}

pub(crate) fn start_bidi(
    channel: Channel,
    path: &str,
    input: OptMsgDesc,
    output: OptMsgDesc,
) -> Gd<GrpcCall> {
    let mut begun = match begin(input, output) {
        Ok(b) => b,
        Err(call) => return call,
    };
    let path = match parse_path(&begun, path) {
        Ok(p) => p,
        Err(()) => return begun.call,
    };
    let requests = attach_outbound(&mut begun.call);
    spawn_and_attach(begun, move |id, sender| async move {
        match transport::open_bidi_stream(channel, path, requests).await {
            Ok(stream) => pump_stream(id, &sender, stream).await,
            Err(status) => {
                let _ = sender.send(fail(id, &status));
            }
        }
    })
}

/// Drain a tonic response stream into `StreamItem` events, then `Completed`.
async fn pump_stream(id: u64, sender: &Sender<PumpEvent>, mut stream: tonic::Streaming<Vec<u8>>) {
    loop {
        match stream.message().await {
            Ok(Some(bytes)) => {
                let _ = sender.send(PumpEvent::StreamItem { call_id: id, bytes });
            }
            Ok(None) => {
                let _ = sender.send(PumpEvent::Completed {
                    call_id: id,
                    bytes: Vec::new(),
                });
                break;
            }
            Err(status) => {
                let _ = sender.send(fail(id, &status));
                break;
            }
        }
    }
}

/// Wire up the outbound request channel for a client-/bidi-streaming call.
fn attach_outbound(call: &mut Gd<GrpcCall>) -> UnboundedReceiverStream<Vec<u8>> {
    let (tx, rx) = unbounded_channel::<Vec<u8>>();
    call.bind_mut().set_outbound(tx);
    UnboundedReceiverStream::new(rx)
}

/// Build a `Failed` event from a `tonic::Status`.
fn fail(call_id: u64, status: &tonic::Status) -> PumpEvent {
    PumpEvent::Failed {
        call_id,
        code: i32::from(status.code()) as i64,
        message: status.message().to_string(),
    }
}
