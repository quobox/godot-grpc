extends Node

# Verifies GrpcCall.cancel(): start an endless server stream, cancel after the
# first item, then confirm `cancelled` fired, `completed` did not, and no
# further items arrive. Requires the fixture test-server on 127.0.0.1:50051.

var _call
var _items := 0
var _cancelled := false
var _completed := false

func _ready() -> void:
	var pool := GrpcDescriptorPool.new()
	if not pool.load_file("res://helloworld.descriptor.bin"):
		_fail("could not load descriptor set")
		return
	var channel := GrpcChannel.tcp("http://127.0.0.1:50051")
	var stub := pool.service("helloworld.Greeter").client(channel)

	_call = stub.server_stream("SayHelloForever", { "name": "world" })
	_call.stream_item.connect(_on_item)
	_call.completed.connect(func(_r): _completed = true)
	_call.cancelled.connect(func(): _cancelled = true)
	_call.failed.connect(func(status): _fail("unexpected failure: " + status.message()))

func _on_item(_message) -> void:
	_items += 1
	if _items == 1:
		_call.cancel()
		# Give any in-flight items a chance to (not) arrive, then check.
		await get_tree().create_timer(0.5).timeout
		_check()

func _check() -> void:
	if not _cancelled:
		_fail("cancelled signal did not fire")
	elif _completed:
		_fail("completed fired after cancel")
	else:
		print("[cancel-test] PASS — cancelled after %d item(s), no completion" % _items)
		get_tree().quit(0)

func _fail(reason: String) -> void:
	print("[cancel-test] FAIL: ", reason)
	get_tree().quit(1)
