extends Node

# Minimal smoke scene for headless extension load/unload verification (M1 gate).
# Prints a marker the test harness can grep for, then quits.

func _ready() -> void:
	print("[godot-grpc] test project ready; extension loaded")
	# Sanity-check that every public class registered with the engine.
	for cls in ["GrpcRuntime", "GrpcChannel", "GrpcCall", "GrpcStatus", "GrpcError"]:
		if not ClassDB.class_exists(cls):
			print("[godot-grpc] FAIL: %s not registered" % cls)
			get_tree().quit(1)
			return
	print("[godot-grpc] all classes registered")
	get_tree().quit()
