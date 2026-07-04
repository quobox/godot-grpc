# godot-grpc

Native gRPC client support for Godot 4.6+, as a Rust GDExtension built on
godot-rust (gdext), tonic, and tokio.

- Unary and all four streaming modes, over TCP or Unix Domain Sockets.
- Call from GDScript with no codegen (tier 2: `Dictionary`/`GrpcMessage`), or
  with typed generated clients (see `protoc-gen-godot-grpc`).
- tokio runs off-thread; results are delivered on the Godot main thread.

The default feature set is tier-1 (raw bytes) only; enable `tier2` for the
runtime descriptor API.

See the [project README](https://github.com/quobox/godot-grpc) and
[guide](https://quobox.github.io/godot-grpc/) for details.
MIT-licensed.
