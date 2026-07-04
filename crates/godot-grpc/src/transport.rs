//! Pure tonic transport helpers — **no Godot** — shared by the Godot classes
//! and the integration tests: lazy channel construction, a passthrough byte
//! codec, and a generic unary call.
//!
//! Tier 1 is wire-bytes in / wire-bytes out: the caller hands an already-encoded
//! request message and receives the encoded response, which it decodes with its
//! own generated prost type. The same generic path underpins tier 2 later, with
//! a `DynamicMessage` codec swapped in for [`BytesCodec`].

use bytes::{Buf, BufMut};
use tonic::client::Grpc;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::codegen::http::uri::PathAndQuery;
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Status};

use crate::runtime;

/// Build a lazily-connecting channel from a URI (`http://…` or `unix://…`).
/// The transport connection is established on first use, on the Tokio runtime.
pub(crate) fn connect_lazy(uri: &str) -> Result<Channel, tonic::transport::Error> {
    let endpoint = Endpoint::try_from(uri.to_string())?;
    // Enter the runtime context so the lazy channel has a reactor when it
    // connects on first use. Harmless if no runtime is running.
    let handle = runtime::handle();
    let _guard = handle.as_ref().map(|h| h.enter());
    Ok(endpoint.connect_lazy())
}

/// Build a generic client and wait until the channel is ready, mapping a
/// not-ready transport error to `UNAVAILABLE`.
async fn ready_client(channel: Channel) -> Result<Grpc<Channel>, Status> {
    let mut grpc = Grpc::new(channel);
    grpc.ready()
        .await
        .map_err(|e| Status::unavailable(format!("channel not ready: {e}")))?;
    Ok(grpc)
}

/// Send one unary RPC over `channel` to `path`, with `request` already encoded.
/// Returns the encoded response bytes, or a `Status` on failure.
pub(crate) async fn unary(
    channel: Channel,
    path: PathAndQuery,
    request: Vec<u8>,
) -> Result<Vec<u8>, Status> {
    let mut grpc = ready_client(channel).await?;
    let response = grpc.unary(Request::new(request), path, BytesCodec).await?;
    Ok(response.into_inner())
}

/// Open a server-streaming RPC. Returns the response stream; the caller pumps it
/// with `.message().await` to receive each encoded message until `None` (end).
pub(crate) async fn open_server_stream(
    channel: Channel,
    path: PathAndQuery,
    request: Vec<u8>,
) -> Result<tonic::Streaming<Vec<u8>>, Status> {
    let mut grpc = ready_client(channel).await?;
    let response = grpc
        .server_streaming(Request::new(request), path, BytesCodec)
        .await?;
    Ok(response.into_inner())
}

/// Run a client-streaming RPC: send the `requests` stream, get one response.
pub(crate) async fn client_streaming<S>(
    channel: Channel,
    path: PathAndQuery,
    requests: S,
) -> Result<Vec<u8>, Status>
where
    S: tokio_stream::Stream<Item = Vec<u8>> + Send + 'static,
{
    let mut grpc = ready_client(channel).await?;
    let response = grpc
        .client_streaming(Request::new(requests), path, BytesCodec)
        .await?;
    Ok(response.into_inner())
}

/// Open a bidirectional-streaming RPC: send the `requests` stream, return the
/// response stream to pump with `.message().await`.
pub(crate) async fn open_bidi_stream<S>(
    channel: Channel,
    path: PathAndQuery,
    requests: S,
) -> Result<tonic::Streaming<Vec<u8>>, Status>
where
    S: tokio_stream::Stream<Item = Vec<u8>> + Send + 'static,
{
    let mut grpc = ready_client(channel).await?;
    let response = grpc
        .streaming(Request::new(requests), path, BytesCodec)
        .await?;
    Ok(response.into_inner())
}

/// A gRPC codec that passes message bodies through as raw bytes (no proto
/// decoding). Encode and decode are both `Vec<u8>`.
#[derive(Default)]
pub(crate) struct BytesCodec;

impl Codec for BytesCodec {
    type Encode = Vec<u8>;
    type Decode = Vec<u8>;
    type Encoder = BytesEncoder;
    type Decoder = BytesDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        BytesEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        BytesDecoder
    }
}

pub(crate) struct BytesEncoder;

impl Encoder for BytesEncoder {
    type Item = Vec<u8>;
    type Error = Status;

    fn encode(&mut self, item: Vec<u8>, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        dst.put_slice(&item);
        Ok(())
    }
}

pub(crate) struct BytesDecoder;

impl Decoder for BytesDecoder {
    type Item = Vec<u8>;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Vec<u8>>, Status> {
        // tonic has already deframed exactly one message into `src`.
        let mut out = vec![0u8; src.remaining()];
        src.copy_to_slice(&mut out);
        Ok(Some(out))
    }
}
