# Installation

`godot-grpc` is a GDExtension: you build a native library and point a
`.gdextension` manifest at it.

## Requirements

- **Rust ≥ 1.94** (Edition 2024)
- **Godot 4.6+**
- **Linux or macOS** (Windows is future work)
- `protoc` (only to produce descriptor sets / run codegen)

## Build the extension

```bash
git clone https://github.com/quobox/godot-grpc
cd godot-grpc
# tier 1 only (smallest binary):
cargo build -p godot-grpc --release
# or with the tier-2 runtime descriptor API:
cargo build -p godot-grpc --features tier2 --release
```

The default feature set is tier-1 only, to keep the binary small. Enable
`tier2` if you want the `Dictionary`/`GrpcMessage` runtime API or use generated
code.

## Add it to your Godot project

Copy the built library into your project (e.g. `res://addons/godot-grpc/lib/`)
and add a `.gdextension` manifest:

```ini
[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.6
reloadable = true

[libraries]
linux.debug.x86_64   = "res://addons/godot-grpc/lib/libgodot_grpc.so"
linux.release.x86_64 = "res://addons/godot-grpc/lib/libgodot_grpc.so"
macos.debug          = "res://addons/godot-grpc/lib/libgodot_grpc.dylib"
macos.release        = "res://addons/godot-grpc/lib/libgodot_grpc.dylib"
```

When developing against a checkout, you can instead point the library paths
directly at `target/debug/` with a relative `res://../../target/debug/...` path.

> **Headless note:** the first time a project loads a new `.gdextension`, run the
> editor once (`godot --headless --editor --path <project> --quit`) so Godot
> records it in `.godot/extension_list.cfg`. After changing registered classes,
> rebuild the library (`cargo build`) before launching Godot, or it loads a
> stale copy.

## Verify

```gdscript
func _ready() -> void:
    print(ClassDB.class_exists("GrpcChannel"))  # true once loaded
```
