# helloworld (GDScript) example

A minimal Godot 4.6 project showing how to call a gRPC service from GDScript via
the tier-2 runtime API (no codegen). It uses `helloworld.v1.GreeterService` from
[proto/helloworld/v1/greeter.proto](proto/helloworld/v1/greeter.proto).

This illustrates the **usage pattern** — it is not wired to a running server.
Point it at any server that implements `helloworld.v1.GreeterService`.

## Setup

From the repository root:

```bash
# 1. Build the extension (tier 2 enables the runtime descriptor API).
cargo build -p godot-grpc --features tier2

# 2. Generate the descriptor set this example loads at runtime.
protoc --descriptor_set_out=examples/helloworld-gdscript/greeter.descriptor.bin \
       -I examples/helloworld-gdscript/proto \
       examples/helloworld-gdscript/proto/helloworld/v1/greeter.proto

# 3. Open/run the project in Godot 4.6, with a GreeterService server reachable
#    at the address in main.gd (default http://127.0.0.1:50051).
godot --path examples/helloworld-gdscript
```

See [main.gd](main.gd) for the code, and the [docs](https://quobox.github.io/godot-grpc/) for the
typed-codegen and streaming APIs.
