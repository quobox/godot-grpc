extends Node

# Headless end-to-end test of the GENERATED typed GDScript client
# (protoc-gen-godot-grpc output in res://generated/). Requires the fixture
# test-server on 127.0.0.1:50051. Validates the typed `await` unary round-trip
# and typed server-streaming.

var _greeter: HelloworldGreeter
var _stream_items: Array = []

func _ready() -> void:
	var channel := GrpcChannel.tcp("http://127.0.0.1:50051")
	if channel == null:
		_fail("GrpcChannel.tcp returned null")
		return
	_greeter = HelloworldGreeter.new(channel)

	var req := HelloworldMessages.HelloRequest.new()
	req.name = "world"

	var reply: HelloworldMessages.HelloReply = await _greeter.say_hello(req)
	if reply == null:
		_fail("say_hello returned null")
		return
	if reply.message != "Hello world":
		_fail("unary: unexpected reply '%s'" % reply.message)
		return
	print("[gen-test] unary OK: ", reply.message)

	var call := _greeter.say_hello_stream(req)
	call.stream_item.connect(func(m): _stream_items.append(HelloworldMessages.HelloReply.wrap(m).message))
	call.completed.connect(func(_r): _on_stream_done())
	call.failed.connect(func(status): _fail("stream failed: " + status.message()))

func _on_stream_done() -> void:
	var expected := ["Hello world #1", "Hello world #2", "Hello world #3"]
	if _stream_items == expected:
		print("[gen-test] stream OK: ", _stream_items)
		print("[gen-test] PASS")
		get_tree().quit(0)
	else:
		_fail("stream items: %s" % str(_stream_items))

func _fail(reason: String) -> void:
	print("[gen-test] FAIL: ", reason)
	get_tree().quit(1)
