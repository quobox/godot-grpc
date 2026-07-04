extends Node

## Minimal godot-grpc example (tier 2 — runtime descriptors, no codegen).
##
## This shows the usage *pattern*; it is not wired to a running server. Point it
## at any server implementing helloworld.v1.GreeterService.
##
## Setup (from the repo root):
##   1. cargo build -p godot-grpc --features tier2
##   2. generate the descriptor set this example loads at runtime:
##        protoc --descriptor_set_out=examples/helloworld-gdscript/greeter.descriptor.bin \
##               -I examples/helloworld-gdscript/proto \
##               examples/helloworld-gdscript/proto/helloworld/v1/greeter.proto
##   3. open/run this project in Godot 4.6 (with your GreeterService server running).

func _ready() -> void:
	var pool := GrpcDescriptorPool.new()
	if not pool.load_file("res://greeter.descriptor.bin"):
		push_error("Missing greeter.descriptor.bin — run protoc (see the header comment).")
		return

	var channel := GrpcChannel.tcp("http://127.0.0.1:50051")
	var stub := pool.service("helloworld.v1.GreeterService").client(channel)

	var call := stub.unary("SayHello", { "name": "Godot" })
	call.completed.connect(func(reply): print("Greeter replied: ", reply.get_field("message")))
	call.failed.connect(func(status): push_error("RPC failed: %s" % status.message()))
