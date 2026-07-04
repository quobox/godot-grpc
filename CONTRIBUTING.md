# Contributing

Thanks for your interest in improving `godot-grpc` — native gRPC client support
for Godot as a Rust GDExtension. Contributions of any size are welcome.

## Development setup

Requires a Rust toolchain (**MSRV 1.94**, Edition 2024) and `protoc` (the
`grpc-test-fixtures` build compiles a `.proto`). Godot 4.6+ is only needed for
the end-to-end scenes.

```bash
git clone https://github.com/quobox/godot-grpc
cd godot-grpc
prek install     # install the pre-commit hooks (https://github.com/j178/prek)
```

`protoc`: `apt-get install protobuf-compiler` (Debian/Ubuntu) or
`brew install protobuf` (macOS).

## Before opening a pull request

The pre-commit hook runs fmt + clippy on every commit; run the full set locally:

```bash
cargo fmt --all -- --check                                            # format
cargo clippy --all-targets -- -D warnings                             # lint (tier 1)
cargo clippy --all-targets --features godot-grpc/tier2 -- -D warnings # lint (tier 2)
cargo test --workspace                                                # transport + codegen tests
```

All must pass — CI runs exactly these on Linux and macOS. The tests need no
Godot install; they exercise the tonic transport and the codegen plugin
directly.

### Godot end-to-end (optional)

The GDScript scenes under `examples/godot-test-project/` exercise the extension
against the in-process fixture server. They need Godot 4.6+ and a tier-2 build:

```bash
cargo build -p godot-grpc --features tier2
./target/debug/test_server 127.0.0.1:50051 &
godot --headless --path examples/godot-test-project res://tier2_test.tscn
```

Each scene prints a `PASS` line. See [the docs](https://quobox.github.io/godot-grpc/) for the full list.

## Guidelines

- **Branch off `master`** and open your pull request against `master`.
- **[Conventional Commits](https://www.conventionalcommits.org/)**, **no merge
  commits**. Keep code **formatted** (rustfmt) and **clippy-clean**
  (`-D warnings`) — both tier 1 and tier 2.
- New features need tests; bug fixes should add a regression test. Most of the
  suite is hardware-free (no Godot required), so you can contribute without it.
- Match the surrounding style: follow the neighbouring code's naming, comment
  density and idiom.
- Keep public-facing files (README, docs, examples) on **generic** endpoints
  (`127.0.0.1`, `/tmp/app.sock`) — no deployment-specific references.
- Add an entry under `## [Unreleased]` in [`CHANGELOG.md`](CHANGELOG.md).

## Scope

The functional client core is complete: unary plus all four streaming modes,
over TCP and Unix Domain Sockets, via tier 1 (raw bytes), tier 2 (runtime
descriptors) and typed GDScript codegen. Not yet implemented and especially
welcome: **TLS**, the **server side** (`GrpcServer`), **reflection**, and
**Windows** support.

## Licensing of contributions

By submitting a contribution you agree that it is licensed under the project's
[MIT License](LICENSE) (inbound = outbound).
