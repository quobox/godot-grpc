# Typed codegen

For the most ergonomic API, `protoc-gen-godot-grpc` generates typed GDScript
classes from your `.proto` files: typed message properties and `await`-able
service methods.

## Generate

Build the plugin, then run `protoc` with it:

```bash
cargo build -p protoc-gen-godot-grpc --release

protoc --plugin=protoc-gen-godot-grpc=target/release/protoc-gen-godot-grpc \
       --godot-grpc_out=res/generated \
       -I proto proto/greeter.proto
```

(The binary must be named `protoc-gen-godot-grpc`; `protoc` finds it via the
`--plugin=` flag or on your `PATH`.)

For `greeter.proto` this emits:

- `greeter_messages.gd` — `class_name GreeterMessages` with one typed inner
  class per message.
- `greeter_greeter.gd` — `class_name GreeterGreeter`, a typed service stub.

The generated message file embeds the schema (as base64) and builds its own
descriptor pool lazily, so the generated code is **self-contained** — no
separate `.descriptor.bin` to ship.

## Use

```gdscript
var greeter := GreeterGreeter.new(GrpcChannel.tcp("http://127.0.0.1:50051"))

var req := GreeterMessages.HelloRequest.new()
req.name = "world"                       # typed setter

var reply := await greeter.say_hello(req)  # typed coroutine
print(reply.message)                      # typed getter
```

- Message fields become typed properties backed by `get_field`/`set_field`.
- **Unary** methods are coroutines: `await greeter.say_hello(req)` returns the
  typed reply.
- **Streaming** methods return a `GrpcCall`; wrap each `stream_item` with the
  generated `Reply.wrap(message)` for typed access.

## Error handling caveat

The generated unary coroutine awaits the `completed` signal. On an RPC failure,
`failed` fires instead and the generated method logs it via `push_error` — but
the awaiting coroutine will not resume. If you need to react to failures in the
`await` path today, use the tier-2 stub directly and connect both
`completed` and `failed`. More ergonomic error handling on the `await` path is
planned.

## Requirements

Generated code uses the tier-2 runtime API, so build the extension with
`--features tier2`.
