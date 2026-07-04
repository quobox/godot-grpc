//! Standalone fixture Greeter server for the GDScript headless integration
//! test. Listens on a TCP address (default `127.0.0.1:50051`, or `argv[1]`).

use grpc_test_fixtures::MyGreeter;
use grpc_test_fixtures::helloworld::greeter_server::GreeterServer;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:50051".to_string())
        .parse()?;
    eprintln!("[test-server] Greeter listening on {addr}");
    Server::builder()
        .add_service(GreeterServer::new(MyGreeter))
        .serve(addr)
        .await?;
    Ok(())
}
