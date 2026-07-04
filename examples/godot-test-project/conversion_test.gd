extends Node

# Exercises the full tier-2 Variant <-> proto conversion by echoing
# a Payload with every field kind through the server and asserting the round
# trip. Requires the fixture test-server on 127.0.0.1:50051.

func _ready() -> void:
	var pool := GrpcDescriptorPool.new()
	if not pool.load_file("res://helloworld.descriptor.bin"):
		_fail("could not load descriptor set")
		return
	var channel := GrpcChannel.tcp("http://127.0.0.1:50051")
	var stub := pool.service("helloworld.Greeter").client(channel)

	var payload := {
		"i": 42,
		"big": 9000000000,            # > 2^31, fits in int64
		"d": 3.5,
		"flag": true,
		"text": "hello",
		"blob": PackedByteArray([1, 2, 3]),
		"color": 2,                   # COLOR_GREEN
		"tags": ["a", "b", "c"],
		"counts": { "x": 1, "y": 2 },
		"nested": { "label": "inner" },
	}
	var call := stub.unary("Echo", payload)
	call.completed.connect(_on_echo)
	call.failed.connect(func(status): _fail("Echo failed: " + status.message()))

func _on_echo(r) -> void:
	var errs: Array = []
	if r.get_field("i") != 42: errs.append("i=%s" % r.get_field("i"))
	if r.get_field("big") != 9000000000: errs.append("big=%s" % r.get_field("big"))
	if absf(r.get_field("d") - 3.5) > 0.0001: errs.append("d=%s" % r.get_field("d"))
	if r.get_field("flag") != true: errs.append("flag")
	if r.get_field("text") != "hello": errs.append("text=%s" % r.get_field("text"))
	if r.get_field("blob") != PackedByteArray([1, 2, 3]): errs.append("blob=%s" % str(r.get_field("blob")))
	if r.get_field("color") != 2: errs.append("color=%s" % r.get_field("color"))
	if r.get_field("tags") != ["a", "b", "c"]: errs.append("tags=%s" % str(r.get_field("tags")))
	var counts = r.get_field("counts")
	if counts.get("x") != 1 or counts.get("y") != 2: errs.append("counts=%s" % str(counts))
	var nested = r.get_field("nested")
	if nested == null or nested.get_field("label") != "inner": errs.append("nested")
	# to_dict() should also reflect the values.
	if r.to_dict().get("text") != "hello": errs.append("to_dict.text")

	if errs.is_empty():
		print("[conv-test] PASS — all field types round-tripped")
		get_tree().quit(0)
	else:
		_fail("mismatches: " + str(errs))

func _fail(reason: String) -> void:
	print("[conv-test] FAIL: ", reason)
	get_tree().quit(1)
