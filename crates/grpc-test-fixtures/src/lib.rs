//! Dev-only test fixtures for godot-grpc.
//!
//! Compiles `proto/helloworld.proto` (via `build.rs` + tonic-prost-build) and
//! exposes an in-process tonic `Greeter` server spawnable over TCP or a Unix
//! Domain Socket, for the integration tests in the `godot-grpc` crate.

pub mod helloworld {
    tonic::include_proto!("helloworld");
}

/// The serialized `FileDescriptorSet` for `helloworld.proto` (for codegen tests).
pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/helloworld.fds"));

use helloworld::greeter_server::{Greeter, GreeterServer};
use helloworld::{HelloReply, HelloRequest, Payload};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[derive(Default)]
pub struct MyGreeter;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let name = request.into_inner().name;
        Ok(Response::new(HelloReply {
            message: format!("Hello {name}"),
        }))
    }

    type SayHelloStreamStream = tokio_stream::wrappers::ReceiverStream<Result<HelloReply, Status>>;

    async fn say_hello_stream(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<Self::SayHelloStreamStream>, Status> {
        let name = request.into_inner().name;
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(async move {
            for i in 1..=3 {
                let reply = HelloReply {
                    message: format!("Hello {name} #{i}"),
                };
                if tx.send(Ok(reply)).await.is_err() {
                    break;
                }
            }
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    async fn say_hello_client_stream(
        &self,
        request: Request<tonic::Streaming<HelloRequest>>,
    ) -> Result<Response<HelloReply>, Status> {
        let mut stream = request.into_inner();
        let mut names = Vec::new();
        while let Some(req) = stream.message().await? {
            names.push(req.name);
        }
        Ok(Response::new(HelloReply {
            message: format!("Hello {}", names.join(", ")),
        }))
    }

    type SayHelloBidiStream = tokio_stream::wrappers::ReceiverStream<Result<HelloReply, Status>>;

    async fn say_hello_bidi(
        &self,
        request: Request<tonic::Streaming<HelloRequest>>,
    ) -> Result<Response<Self::SayHelloBidiStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(async move {
            while let Ok(Some(req)) = stream.message().await {
                let reply = HelloReply {
                    message: format!("Hello {}", req.name),
                };
                if tx.send(Ok(reply)).await.is_err() {
                    break;
                }
            }
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    async fn echo(&self, request: Request<Payload>) -> Result<Response<Payload>, Status> {
        Ok(Response::new(request.into_inner()))
    }

    async fn always_fails(
        &self,
        _request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        Err(Status::invalid_argument("this RPC always fails"))
    }

    type SayHelloForeverStream = tokio_stream::wrappers::ReceiverStream<Result<HelloReply, Status>>;

    async fn say_hello_forever(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<Self::SayHelloForeverStream>, Status> {
        let name = request.into_inner().name;
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(async move {
            let mut i = 0u64;
            loop {
                i += 1;
                let reply = HelloReply {
                    message: format!("Hello {name} #{i}"),
                };
                if tx.send(Ok(reply)).await.is_err() {
                    break; // client dropped / cancelled
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }
}

/// Spawn the Greeter on an ephemeral TCP port. Must be called within a Tokio
/// runtime context. Returns the bound `127.0.0.1` address.
pub async fn spawn_tcp() -> std::net::SocketAddr {
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let incoming = TcpListenerStream::new(listener);
    tokio::spawn(async move {
        Server::builder()
            .add_service(GreeterServer::new(MyGreeter))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });
    addr
}

/// Spawn the Greeter on a Unix Domain Socket at `path`. Must be called within a
/// Tokio runtime context. Removes any stale socket file first.
#[cfg(unix)]
pub async fn spawn_uds(path: impl AsRef<std::path::Path>) {
    use tokio::net::UnixListener;
    use tokio_stream::wrappers::UnixListenerStream;

    let path = path.as_ref().to_path_buf();
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).unwrap();
    let incoming = UnixListenerStream::new(listener);
    tokio::spawn(async move {
        Server::builder()
            .add_service(GreeterServer::new(MyGreeter))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });
}
