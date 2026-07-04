# godot-grpc

Native gRPC client support for [Godot 4.6+](https://godotengine.org), as a Rust
[GDExtension](https://docs.godotengine.org/en/stable/tutorials/scripting/gdextension/what_is_gdextension.html)
built on [godot-rust (gdext)](https://github.com/godot-rust/gdext),
[tonic](https://github.com/hyperium/tonic), and [tokio](https://tokio.rs).

Godot has no built-in gRPC support. `godot-grpc` fills the gap with type-safe
RPCs, all four streaming modes, and Unix Domain Socket transport — usable
directly from GDScript with no code generation, or with generated typed clients.

> **Status:** functional core complete (unary + streaming, runtime + generated
> APIs). Pre-1.0; APIs may still change. MIT-licensed.

## Features

- **Unary and streaming** RPCs — server-, client-, and bidirectional-streaming.
- **TCP and Unix Domain Sockets** (`http://host:port` or `unix:///path/to.sock`).
- **Two ways to call**, no build step required for either:
  - **Tier 2 (runtime):** load a `.descriptor.bin`, call services with
    `Dictionary` requests and `GrpcMessage` responses.
  - **Typed codegen:** `protoc-gen-godot-grpc` emits typed GDScript classes
    (`req.name = "world"`, `await greeter.say_hello(req)`).
  - **Tier 1 (raw bytes):** lowest-level API for advanced/embedded use.
- **Sound threading:** tokio runs off-thread; results are marshalled to the
  Godot main thread — no `experimental-threads`, no unsafe `Gd<T>` sharing.

## Quick start (tier 2, no codegen)

1. Build the extension and add it to your project (see [the docs](https://quobox.github.io/godot-grpc/)):
   ```bash
   cargo build -p godot-grpc --features tier2 --release
   ```
2. Export a descriptor set for your protos:
   ```bash
   protoc --descriptor_set_out=res/greeter.descriptor.bin -I proto proto/greeter.proto
   ```
3. Call it from GDScript:
   ```gdscript
   var pool := GrpcDescriptorPool.new()
   pool.load_file("res://greeter.descriptor.bin")

   var channel := GrpcChannel.tcp("http://127.0.0.1:50051")
   var stub := pool.service("greeter.Greeter").client(channel)

   var call := stub.unary("SayHello", { "name": "world" })
   call.completed.connect(func(reply): print(reply.get_field("message")))
   call.failed.connect(func(status): push_error(status.message()))
   ```

## Quick start (typed codegen)

```bash
cargo build -p protoc-gen-godot-grpc --release
protoc --plugin=protoc-gen-godot-grpc=target/release/protoc-gen-godot-grpc \
       --godot-grpc_out=res/generated -I proto proto/greeter.proto
```
```gdscript
var greeter := GreeterGreeter.new(GrpcChannel.tcp("http://127.0.0.1:50051"))
var req := GreeterMessages.HelloRequest.new()
req.name = "world"
var reply := await greeter.say_hello(req)
print(reply.message)
```

## Workspace layout

| Crate | What |
|---|---|
| [`godot-grpc`](crates/godot-grpc) | The GDExtension (cdylib). Default build is tier-1 only; `--features tier2` adds the runtime descriptor API. |
| [`protoc-gen-godot-grpc`](crates/protoc-gen-godot-grpc) | protoc plugin generating typed GDScript clients. |
| [`grpc-test-fixtures`](crates/grpc-test-fixtures) | Dev-only test server + proto fixtures. |

## Building & testing

```bash
cargo build --features tier2
cargo clippy --all-targets --features tier2
cargo test                      # transport + codegen tests
```
See [the docs](https://quobox.github.io/godot-grpc/) for the headless GDScript
end-to-end tests and the full guide.

## Requirements

- Rust ≥ 1.94 (Edition 2024) · Godot 4.6+ · Linux/macOS (Windows is future work).

## Distribution

Intended channels (not yet published — pre-1.0):

- **crates.io** for the source crate `godot-grpc` (path-1 Rust authors who depend
  on it as a Cargo crate) and the `protoc-gen-godot-grpc` plugin
  (`cargo install protoc-gen-godot-grpc`).
- **Godot AssetLib** for GDScript users, shipping precompiled per-platform
  libraries (they don't build Rust) plus the addon manifest.

Both crates pass `cargo publish --dry-run`. `grpc-test-fixtures` is dev-only
(`publish = false`).

## License

MIT — see [LICENSE](LICENSE).
