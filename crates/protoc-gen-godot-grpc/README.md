# protoc-gen-godot-grpc

A `protoc` plugin that generates typed GDScript clients for
[godot-grpc](https://github.com/quobox/godot-grpc).

```bash
cargo install protoc-gen-godot-grpc
protoc --godot-grpc_out=res/generated -I proto proto/greeter.proto
```

For each `.proto` it emits a typed message file (classes extending
`GrpcMessage` with typed properties, schema embedded as base64) and a typed
service stub (`await`-able unary methods, streaming returning `GrpcCall`).
Generated code uses godot-grpc's tier-2 runtime API.

See the [guide](https://quobox.github.io/godot-grpc/) for
usage. MIT-licensed.
