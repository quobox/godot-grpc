# The two tiers

`godot-grpc` exposes the same transport through two layers. Both run over the
identical channel/runtime/bridge; they differ only in how messages are
represented on the GDScript side.

| | Tier 1 | Tier 2 |
|---|---|---|
| Request/response | `PackedByteArray` (encoded) | `Dictionary` / `GrpcMessage` |
| Schema needed at runtime | no | yes (a `FileDescriptorSet`) |
| Build feature | always | `--features tier2` |
| Best for | Rust authors with their own generated clients; embedded use | GDScript users; the typed codegen builds on this |

## Tier 1 — raw bytes

The lowest level. You hand `GrpcChannel` an already-encoded request and receive
encoded response bytes; you decode them yourself (e.g. with a generated prost
type on the Rust side, or a GDScript protobuf library).

```gdscript
var channel := GrpcChannel.tcp("http://127.0.0.1:50051")
var call := channel.unary_call("/greeter.Greeter/SayHello", request_bytes)
call.completed.connect(func(bytes: PackedByteArray): ...)  # decode yourself
```

The method path is `/<package>.<Service>/<Method>`. Streaming uses
`server_stream_call`, `client_stream_call`, and `bidi_call`.

## Tier 2 — runtime descriptors

Load a `FileDescriptorSet` into a `GrpcDescriptorPool` and call services by
name with `Dictionary` requests; responses come back as `GrpcMessage` objects
with `get_field` / `set_field` / `to_dict`.

```gdscript
var pool := GrpcDescriptorPool.new()
pool.load_file("res://greeter.descriptor.bin")
var channel := GrpcChannel.tcp("http://127.0.0.1:50051")
var stub := pool.service("greeter.Greeter").client(channel)
var call := stub.unary("SayHello", { "name": "world" })
call.completed.connect(func(reply): print(reply.get_field("message")))
```

`GrpcServiceStub` methods: `unary`, `server_stream`, `client_stream`, `bidi`
(named `unary` — not `call` — because `Object.call` is a Godot built-in).

## The result of either is a `GrpcCall`

Every call returns a `GrpcCall` with these signals:

- `completed(response)` — success (unary) or end-of-stream.
- `stream_item(message)` — one server-/bidi-stream message.
- `failed(status: GrpcStatus)` — a gRPC or transport error.
- `cancelled()`.

In tier 1 the payloads are `PackedByteArray`; in tier 2 they are `GrpcMessage`.
