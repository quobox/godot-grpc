extends Node

# Headless end-to-end test of the Godot-side gRPC path. Requires the fixture
# test-server on 127.0.0.1:50051. Runs a unary call, then a server-streaming
# call (verifies stream_item fires repeatedly and the call stays registered
# until completed).

var _channel
var _unary
var _stream
var _items: Array = []

func _ready() -> void:
	_channel = GrpcChannel.tcp("http://127.0.0.1:50051")
	if _channel == null:
		_fail("GrpcChannel.tcp returned null")
		return
	_unary = _channel.unary_call("/helloworld.Greeter/SayHello", _hello("world"))
	_unary.completed.connect(_on_unary)
	_unary.failed.connect(_on_failed)

# Hand-encode HelloRequest { name }: field 1, wire type 2 -> tag 0x0A, len, bytes.
func _hello(name: String) -> PackedByteArray:
	var n := name.to_utf8_buffer()
	var req := PackedByteArray([0x0A, n.size()])
	req.append_array(n)
	return req

# Decode HelloReply { message }: skip the 2-byte header (tag, length).
func _decode(bytes: PackedByteArray) -> String:
	return bytes.slice(2).get_string_from_utf8()

func _on_unary(response: PackedByteArray) -> void:
	var msg := _decode(response)
	if msg != "Hello world":
		_fail("unary: unexpected reply '%s'" % msg)
		return
	print("[grpc-test] unary OK: ", msg)
	_stream = _channel.server_stream_call("/helloworld.Greeter/SayHelloStream", _hello("world"))
	_stream.stream_item.connect(_on_item)
	_stream.completed.connect(_on_stream_done)
	_stream.failed.connect(_on_failed)

func _on_item(message: PackedByteArray) -> void:
	_items.append(_decode(message))

func _on_stream_done(_response: PackedByteArray) -> void:
	var expected := ["Hello world #1", "Hello world #2", "Hello world #3"]
	if _items == expected:
		print("[grpc-test] stream OK: ", _items)
		print("[grpc-test] PASS")
		get_tree().quit(0)
	else:
		_fail("stream items: %s" % str(_items))

func _on_failed(status) -> void:
	_fail("RPC failed: code %d, message '%s'" % [status.code(), status.message()])

func _fail(reason: String) -> void:
	print("[grpc-test] FAIL: ", reason)
	get_tree().quit(1)
