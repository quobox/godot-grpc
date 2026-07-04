# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Pre-1.0: the functional client core is complete, but the public API may still
change before the first tagged release.

### Added

- **Unary and streaming RPCs** — server-, client-, and bidirectional-streaming.
- **TCP and Unix Domain Socket** transports (`http://host:port` or
  `unix:///path/to.sock`).
- **Tier 1 (raw bytes):** `GrpcChannel.unary_call` / `*_call` with
  `PackedByteArray` request/response — the lowest-level API.
- **Tier 2 (runtime descriptors, feature `tier2`):** load a `FileDescriptorSet`
  into `GrpcDescriptorPool`, then call services by name with `Dictionary`
  requests and `GrpcMessage` responses. Zero codegen, backed by `prost-reflect`.
- **Typed GDScript codegen:** `protoc-gen-godot-grpc` emits typed classes over
  tier 2 (`req.name = "world"; await greeter.say_hello(req)`).
- **`GrpcCall` signals** — `completed`, `stream_item`, `failed`, `cancelled`,
  and a unified `finished` that fires exactly once. RPC cancellation is
  supported.
- **Sound threading** — tokio runs off-thread; results are marshalled to the
  Godot main thread over a crossbeam channel, so no `experimental-threads` and
  no unsafe `Gd<T>` sharing.

[Unreleased]: https://github.com/quobox/godot-grpc/commits/master
