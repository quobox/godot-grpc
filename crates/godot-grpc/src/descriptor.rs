//! `GrpcDescriptorPool`: loads a protobuf `FileDescriptorSet` (`.descriptor.bin`)
//! at runtime so GDScript can call gRPC services with zero codegen (tier 2).
//! Backed by `prost_reflect::DescriptorPool`.

use godot::classes::FileAccess;
use godot::prelude::*;
use prost_reflect::{DescriptorPool, MessageDescriptor};

use crate::service::GrpcServiceStub;

#[derive(GodotClass)]
#[class(init, base = RefCounted)]
pub struct GrpcDescriptorPool {
    pool: Option<DescriptorPool>,
    base: Base<RefCounted>,
}

#[godot_api]
impl GrpcDescriptorPool {
    /// Load a serialized `FileDescriptorSet` from raw bytes. Returns false (and
    /// logs) on parse failure.
    #[func]
    fn load_bytes(&mut self, bytes: PackedByteArray) -> bool {
        match DescriptorPool::decode(bytes.as_slice()) {
            Ok(pool) => {
                self.pool = Some(pool);
                true
            }
            Err(e) => {
                godot_error!("[godot-grpc] failed to parse descriptor set: {e}");
                false
            }
        }
    }

    /// Load a `FileDescriptorSet` from a Godot resource path (e.g.
    /// `"res://protos/helloworld.descriptor.bin"`). Returns false on failure.
    #[func]
    fn load_file(&mut self, res_path: GString) -> bool {
        let bytes = FileAccess::get_file_as_bytes(&res_path);
        if bytes.is_empty() {
            godot_error!("[godot-grpc] could not read descriptor file {res_path}");
            return false;
        }
        self.load_bytes(bytes)
    }

    /// Look up a service by its fully-qualified name (e.g.
    /// `"helloworld.Greeter"`). Returns `null` if not loaded or not found.
    #[func]
    fn service(&self, full_name: GString) -> Option<Gd<GrpcServiceStub>> {
        let name = full_name.to_string();
        let Some(pool) = self.pool.as_ref() else {
            godot_error!("[godot-grpc] descriptor pool is empty; call load_file/load_bytes first");
            return None;
        };
        match pool.get_service_by_name(&name) {
            Some(svc) => Some(GrpcServiceStub::create(svc)),
            None => {
                godot_error!("[godot-grpc] service {name:?} not found in descriptor pool");
                None
            }
        }
    }
}

impl GrpcDescriptorPool {
    /// Look up a message descriptor by fully-qualified name (for `GrpcMessage::bind_type`).
    pub(crate) fn message_descriptor(&self, name: &str) -> Option<MessageDescriptor> {
        self.pool.as_ref()?.get_message_by_name(name)
    }
}
