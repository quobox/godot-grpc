//! `protoc-gen-godot-grpc`: a protoc plugin emitting typed GDScript clients.
//!
//! protoc invokes it as `protoc --godot-grpc_out=DIR foo.proto` (the binary must
//! be on `PATH` as `protoc-gen-godot-grpc`). It reads a `CodeGeneratorRequest`
//! from stdin and writes a `CodeGeneratorResponse` to stdout.

use std::io::{Read, Write};

use prost::Message;
use prost_types::compiler::CodeGeneratorRequest;

mod codegen;

fn main() -> std::io::Result<()> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;

    let request = match CodeGeneratorRequest::decode(buf.as_slice()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("protoc-gen-godot-grpc: invalid CodeGeneratorRequest: {e}");
            std::process::exit(1);
        }
    };

    let response = codegen::generate(request);

    let mut out = Vec::new();
    response
        .encode(&mut out)
        .expect("failed to encode CodeGeneratorResponse");
    std::io::stdout().write_all(&out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::codegen;
    use prost::Message;
    use prost_types::FileDescriptorSet;
    use prost_types::compiler::CodeGeneratorRequest;

    fn generate_helloworld() -> Vec<prost_types::compiler::code_generator_response::File> {
        let fds = FileDescriptorSet::decode(grpc_test_fixtures::FILE_DESCRIPTOR_SET)
            .expect("decode fixture FileDescriptorSet");
        let request = CodeGeneratorRequest {
            file_to_generate: vec!["helloworld.proto".to_string()],
            proto_file: fds.file,
            ..Default::default()
        };
        codegen::generate(request).file
    }

    fn content<'a>(
        files: &'a [prost_types::compiler::code_generator_response::File],
        name: &str,
    ) -> &'a str {
        files
            .iter()
            .find(|f| f.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("missing generated file {name}"))
            .content
            .as_deref()
            .unwrap()
    }

    #[test]
    fn generates_typed_messages() {
        let files = generate_helloworld();
        let msgs = content(&files, "helloworld_messages.gd");
        assert!(msgs.contains("class_name HelloworldMessages"));
        assert!(msgs.contains("class HelloRequest extends GrpcMessage:"));
        assert!(msgs.contains("class HelloReply extends GrpcMessage:"));
        assert!(msgs.contains("bind_type(HelloworldMessages.pool(), \"helloworld.HelloRequest\")"));
        assert!(msgs.contains("var name: String:"));
        assert!(msgs.contains("var message: String:"));
        assert!(msgs.contains("const _DESCRIPTOR_B64 :="));
    }

    #[test]
    fn generates_typed_service() {
        let files = generate_helloworld();
        let svc = content(&files, "helloworld_greeter.gd");
        assert!(svc.contains("class_name HelloworldGreeter"));
        assert!(svc.contains(
            "func say_hello(request: HelloworldMessages.HelloRequest) -> HelloworldMessages.HelloReply:"
        ));
        assert!(svc.contains("var result = await call.finished"));
        assert!(svc.contains(
            "func say_hello_stream(request: HelloworldMessages.HelloRequest) -> GrpcCall:"
        ));
    }
}
