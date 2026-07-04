# Introduction

`godot-grpc` brings native [gRPC](https://grpc.io) to [Godot 4.6+](https://godotengine.org)
as a Rust [GDExtension](https://docs.godotengine.org/en/stable/tutorials/scripting/gdextension/what_is_gdextension.html),
built on [godot-rust (gdext)](https://github.com/godot-rust/gdext),
[tonic](https://github.com/hyperium/tonic), and [tokio](https://tokio.rs).

Godot has no built-in gRPC support. Existing community options cover protobuf
*messages* in GDScript but not gRPC *services* or streaming. `godot-grpc` provides:

- **Unary and streaming RPCs** — server-, client-, and bidirectional-streaming.
- **TCP and Unix Domain Sockets** — `http://host:port` or `unix:///path/to.sock`.
- **Three ways to call**, none requiring a Godot-side build step:
  - **Typed codegen** — generated GDScript classes for the most ergonomic API.
  - **Tier 2 (runtime descriptors)** — load a descriptor file, call with
    `Dictionary`/`GrpcMessage`. Zero codegen.
  - **Tier 1 (raw bytes)** — the lowest-level building block.
- **Sound threading** — tokio runs on a background thread; results are delivered
  on the Godot main thread, so `Gd<T>` is never shared across threads.

## Who this is for

- **GDScript developers** who want to talk to a gRPC backend from a Godot game,
  tool, or kiosk app — use tier 2 or the typed codegen.
- **Rust extension authors** who depend on `godot-grpc` as a crate and want the
  channel/runtime/bridge while using their own generated tonic clients (tier 1).

## Status

The functional core is complete and tested (unary + all four streaming modes,
runtime + generated APIs). It is pre-1.0 — APIs may change before 1.0. Linux and
macOS are supported; Windows is future work. MIT-licensed.
