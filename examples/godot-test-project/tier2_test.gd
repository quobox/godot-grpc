extends Node

# Headless end-to-end test of the tier-2 (runtime descriptor) path: load a
# FileDescriptorSet, build a service stub, and call methods with Dictionary
# requests / GrpcMessage responses (no codegen). Requires the fixture
# test-server on 127.0.0.1:50051.

var _stub
var _unary
var _stream
var _items: Array = []

func _ready() -> void:
	var pool := GrpcDescriptorPool.new()
	if not pool.load_file("res://helloworld.descriptor.bin"):
		_fail("could not load descriptor set")
		return

	var channel := GrpcChannel.tcp("http://127.0.0.1:50051")
	if channel == null:
		_fail("GrpcChannel.tcp returned null")
		return

	var service = pool.service("helloworld.Greeter")
	if service == null:
		_fail("service helloworld.Greeter not found")
		return
	_stub = service.client(channel)

	_unary = _stub.unary("SayHello", { "name": "world" })
	if _unary == null:
		_fail("unary() returned null")
		return
	_unary.completed.connect(_on_unary)
	_unary.failed.connect(_on_failed)

func _on_unary(result) -> void:
	var msg = result.get_field("message")
	if msg != "Hello world":
		_fail("unary: unexpected reply '%s'" % msg)
		return
	print("[tier2-test] unary OK: ", msg)

	_stream = _stub.server_stream("SayHelloStream", { "name": "world" })
	if _stream == null:
		_fail("server_stream() returned null")
		return
	_stream.stream_item.connect(_on_item)
	_stream.completed.connect(_on_stream_done)
	_stream.failed.connect(_on_failed)

func _on_item(message) -> void:
	_items.append(message.get_field("message"))

func _on_stream_done(_response) -> void:
	var expected := ["Hello world #1", "Hello world #2", "Hello world #3"]
	if _items == expected:
		print("[tier2-test] stream OK: ", _items)
		print("[tier2-test] PASS")
		get_tree().quit(0)
	else:
		_fail("stream items: %s" % str(_items))

func _on_failed(status) -> void:
	_fail("RPC failed: code %d, message '%s'" % [status.code(), status.message()])

func _fail(reason: String) -> void:
	print("[tier2-test] FAIL: ", reason)
	get_tree().quit(1)
