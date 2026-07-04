# Quick start

This walks through calling a `Greeter` service with a unary `SayHello` RPC,
using the **tier-2 runtime API** (no codegen). Assume this proto:

```proto
syntax = "proto3";
package greeter;

service Greeter {
  rpc SayHello (HelloRequest) returns (HelloReply);
}
message HelloRequest { string name = 1; }
message HelloReply   { string message = 1; }
```

## 1. Export a descriptor set

`godot-grpc` reads your schema at runtime from a serialized `FileDescriptorSet`:

```bash
protoc --descriptor_set_out=greeter.descriptor.bin -I proto proto/greeter.proto
```

Place `greeter.descriptor.bin` in your project, e.g. `res://greeter.descriptor.bin`.

## 2. Call it from GDScript

```gdscript
extends Node

func _ready() -> void:
    var pool := GrpcDescriptorPool.new()
    pool.load_file("res://greeter.descriptor.bin")

    var channel := GrpcChannel.tcp("http://127.0.0.1:50051")
    var stub := pool.service("greeter.Greeter").client(channel)

    var call := stub.unary("SayHello", { "name": "world" })
    call.completed.connect(func(reply): print(reply.get_field("message")))
    call.failed.connect(func(status): push_error(status.message()))
```

That's the whole flow: load schema → open channel → bind a service stub →
call with a `Dictionary`, handle the `completed`/`failed` signals.

## Unix Domain Sockets

Swap the transport — everything else is identical:

```gdscript
var channel := GrpcChannel.uds("/tmp/greeter.sock")
```

## Connections are lazy

`GrpcChannel.tcp(...)` / `.uds(...)` return immediately; the connection is
established on the first RPC. A bad address or unreachable server surfaces as a
`failed(status)` signal on the call, not at channel-construction time.

## Next steps

- Prefer typed `req.name = "world"` over dicts? See [Typed codegen](./codegen.md).
- Streaming RPCs: see [Streaming](./streaming.md).
