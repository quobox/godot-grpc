# Threading model

This is the most important thing to understand about how `godot-grpc` works
internally — and why it's safe.

## The rule

**Godot APIs are only ever touched on the main thread.** tonic needs a full
tokio runtime, which runs on its own background threads. `godot-grpc` keeps
these worlds strictly separated:

- A multi-threaded **tokio runtime** is owned by the extension and started when
  Godot finishes initializing, shut down on exit.
- RPCs run as tokio tasks on that runtime. When a task produces a result, it
  sends **plain Rust data** (encoded bytes + status) over a lock-free
  [`crossbeam-channel`](https://docs.rs/crossbeam-channel) — never a `Gd<T>`.
- Once per frame, on the **main thread**, `godot-grpc` drains that channel and
  emits the `completed` / `stream_item` / `failed` signals on the right
  `GrpcCall`.

Because `Gd<T>` (Godot object handles) never cross to the tokio thread, the
extension needs no `experimental-threads` feature and contains no unsound
cross-thread sharing.

## What this means for you

- **Signals fire on the main thread**, so your handlers can freely touch nodes,
  the scene tree, the UI — no marshalling needed on your side.
- There is up to **one frame of latency** between a response arriving on the
  network and the signal firing (it's delivered on the next frame's drain). For
  the vast majority of game/tool/kiosk use this is irrelevant.
- The runtime is created/destroyed with the extension lifecycle, including
  surviving editor hot-reload.

## Why not `godot::task` / coroutines?

gdext's async integration is great for engine-driven async, but it is not a
tokio runtime and cannot drive tonic's tower-based stack. The two coexist
deliberately: tokio does the networking; the per-frame drain bridges results
back into Godot's signal/await world (which is what makes
`await call.completed` work from GDScript).
