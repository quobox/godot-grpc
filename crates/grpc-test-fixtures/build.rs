fn main() -> Result<(), Box<dyn std::error::Error>> {
    // tonic 0.14: codegen lives in tonic-prost-build; method is `compile_protos`.
    // Emit a FileDescriptorSet to OUT_DIR (exposed as a const for the codegen
    // snapshot test) and copy it into the test project for the tier-2 GDScript
    // test to load.
    let out_dir = std::env::var("OUT_DIR")?;
    let fds_path = format!("{out_dir}/helloworld.fds");
    tonic_prost_build::configure()
        .build_client(true)
        .build_server(true)
        .file_descriptor_set_path(&fds_path)
        .compile_protos(&["proto/helloworld.proto"], &["proto"])?;
    std::fs::copy(
        &fds_path,
        "../../examples/godot-test-project/helloworld.descriptor.bin",
    )?;
    Ok(())
}
