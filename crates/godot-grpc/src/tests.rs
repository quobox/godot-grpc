//! In-crate integration tests for the pure transport path.
//!
//! These drive a real unary RPC through the same `transport::unary` code path
//! used by `GrpcChannel`, against the in-process fixture `Greeter` server over
//! both TCP and a Unix Domain Socket. No Godot engine is involved (the Godot
//! signal-emission side is covered by a headless GDScript scene).

use grpc_test_fixtures::helloworld::{HelloReply, HelloRequest};
use prost::Message;

use crate::transport;

const METHOD: &str = "/helloworld.Greeter/SayHello";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unary_unreachable_is_unavailable() {
    // Nothing listening here → the RPC fails with UNAVAILABLE, not a hang.
    let channel = transport::connect_lazy("http://127.0.0.1:59999").expect("build channel");
    let request = HelloRequest { name: "x".into() }.encode_to_vec();
    let err = transport::unary(channel, METHOD.parse().unwrap(), request)
        .await
        .expect_err("call to a dead address must fail");
    assert_eq!(err.code(), tonic::Code::Unavailable);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unary_error_status_surfaces() {
    let addr = grpc_test_fixtures::spawn_tcp().await;
    let channel = transport::connect_lazy(&format!("http://{addr}")).expect("build channel");
    let request = HelloRequest { name: "x".into() }.encode_to_vec();
    let err = transport::unary(
        channel,
        "/helloworld.Greeter/AlwaysFails".parse().unwrap(),
        request,
    )
    .await
    .expect_err("AlwaysFails must return an error status");
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert_eq!(err.message(), "this RPC always fails");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unary_over_tcp() {
    let addr = grpc_test_fixtures::spawn_tcp().await;
    let channel = transport::connect_lazy(&format!("http://{addr}")).expect("build channel");

    let request = HelloRequest {
        name: "world".into(),
    }
    .encode_to_vec();
    let bytes = transport::unary(channel, METHOD.parse().unwrap(), request)
        .await
        .expect("unary call succeeds");

    let reply = HelloReply::decode(bytes.as_slice()).expect("decode reply");
    assert_eq!(reply.message, "Hello world");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_stream_over_tcp() {
    let addr = grpc_test_fixtures::spawn_tcp().await;
    let channel = transport::connect_lazy(&format!("http://{addr}")).expect("build channel");

    let request = HelloRequest {
        name: "world".into(),
    }
    .encode_to_vec();
    let mut stream = transport::open_server_stream(
        channel,
        "/helloworld.Greeter/SayHelloStream".parse().unwrap(),
        request,
    )
    .await
    .expect("open stream");

    let mut messages = Vec::new();
    while let Some(bytes) = stream.message().await.expect("stream message") {
        messages.push(
            HelloReply::decode(bytes.as_slice())
                .expect("decode")
                .message,
        );
    }
    assert_eq!(
        messages,
        vec!["Hello world #1", "Hello world #2", "Hello world #3"]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn client_stream_over_tcp() {
    let addr = grpc_test_fixtures::spawn_tcp().await;
    let channel = transport::connect_lazy(&format!("http://{addr}")).expect("build channel");

    let requests = tokio_stream::iter(
        ["a", "b", "c"]
            .into_iter()
            .map(|n| HelloRequest { name: n.into() }.encode_to_vec()),
    );
    let bytes = transport::client_streaming(
        channel,
        "/helloworld.Greeter/SayHelloClientStream".parse().unwrap(),
        requests,
    )
    .await
    .expect("client-streaming call succeeds");

    let reply = HelloReply::decode(bytes.as_slice()).expect("decode reply");
    assert_eq!(reply.message, "Hello a, b, c");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bidi_over_tcp() {
    let addr = grpc_test_fixtures::spawn_tcp().await;
    let channel = transport::connect_lazy(&format!("http://{addr}")).expect("build channel");

    let requests = tokio_stream::iter(
        ["x", "y"]
            .into_iter()
            .map(|n| HelloRequest { name: n.into() }.encode_to_vec()),
    );
    let mut stream = transport::open_bidi_stream(
        channel,
        "/helloworld.Greeter/SayHelloBidi".parse().unwrap(),
        requests,
    )
    .await
    .expect("open bidi stream");

    let mut messages = Vec::new();
    while let Some(bytes) = stream.message().await.expect("stream message") {
        messages.push(
            HelloReply::decode(bytes.as_slice())
                .expect("decode")
                .message,
        );
    }
    assert_eq!(messages, vec!["Hello x", "Hello y"]);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unary_over_uds() {
    let sock = std::env::temp_dir().join(format!("godot-grpc-test-{}.sock", std::process::id()));
    grpc_test_fixtures::spawn_uds(&sock).await;
    let channel =
        transport::connect_lazy(&format!("unix://{}", sock.display())).expect("build channel");

    let request = HelloRequest { name: "uds".into() }.encode_to_vec();
    let bytes = transport::unary(channel, METHOD.parse().unwrap(), request)
        .await
        .expect("unary call succeeds");

    let reply = HelloReply::decode(bytes.as_slice()).expect("decode reply");
    assert_eq!(reply.message, "Hello uds");

    let _ = std::fs::remove_file(&sock);
}
