//! `GrpcServiceStub`: a tier-2 service handle that dispatches RPCs by method
//! name with `Dictionary` / `GrpcMessage` payloads.
//!
//! The request `Dictionary` is encoded to wire bytes here (main thread) via the
//! method's input descriptor; the call carries the output descriptor so
//! responses decode back into `GrpcMessage`s. Actual transport reuses the same
//! tier-1 `channel::start_*` machinery.

use godot::prelude::*;
use prost::Message;
use prost_reflect::{MethodDescriptor, ServiceDescriptor};
use tonic::transport::Channel;

use crate::call::GrpcCall;
use crate::channel::{self, GrpcChannel};
use crate::convert;

#[derive(GodotClass)]
#[class(no_init, base = RefCounted)]
pub struct GrpcServiceStub {
    service: ServiceDescriptor,
    channel: Option<Gd<GrpcChannel>>,
    base: Base<RefCounted>,
}

#[godot_api]
impl GrpcServiceStub {
    /// The fully-qualified service name, e.g. `"helloworld.Greeter"`.
    #[func]
    fn full_name(&self) -> GString {
        GString::from(self.service.full_name())
    }

    /// Bind this service to a channel, returning a stub ready to make calls.
    #[func]
    fn client(&self, channel: Gd<GrpcChannel>) -> Gd<GrpcServiceStub> {
        Gd::from_init_fn(|base| GrpcServiceStub {
            service: self.service.clone(),
            channel: Some(channel),
            base,
        })
    }

    /// Unary call. `request` is a `Dictionary` matching the method's input
    /// message; the `completed` signal delivers a `GrpcMessage`. Returns `null`
    /// on misuse (unbound stub, unknown method, bad request).
    ///
    /// (Named `unary`, not `call`, because `Object.call` is a Godot built-in.)
    #[func]
    fn unary(&self, method: GString, request: VarDictionary) -> Option<Gd<GrpcCall>> {
        let (m, channel) = self.prepare(&method.to_string())?;
        let request = self.encode_request(&m, &request)?;
        Some(channel::start_unary(
            channel,
            &method_path(&self.service, &m),
            request,
            Some(m.output()),
        ))
    }

    /// Server-streaming call: responses arrive as `stream_item` `GrpcMessage`s.
    #[func]
    fn server_stream(&self, method: GString, request: VarDictionary) -> Option<Gd<GrpcCall>> {
        let (m, channel) = self.prepare(&method.to_string())?;
        let request = self.encode_request(&m, &request)?;
        Some(channel::start_server_stream(
            channel,
            &method_path(&self.service, &m),
            request,
            Some(m.output()),
        ))
    }

    /// Client-streaming call: send requests with `GrpcCall.send_dict()`, finish
    /// with `close_send()`; the single response arrives as `completed`.
    #[func]
    fn client_stream(&self, method: GString) -> Option<Gd<GrpcCall>> {
        let (m, channel) = self.prepare(&method.to_string())?;
        Some(channel::start_client_stream(
            channel,
            &method_path(&self.service, &m),
            Some(m.input()),
            Some(m.output()),
        ))
    }

    /// Bidirectional call: send with `send_dict()`/`close_send()`, responses as
    /// `stream_item` `GrpcMessage`s.
    #[func]
    fn bidi(&self, method: GString) -> Option<Gd<GrpcCall>> {
        let (m, channel) = self.prepare(&method.to_string())?;
        Some(channel::start_bidi(
            channel,
            &method_path(&self.service, &m),
            Some(m.input()),
            Some(m.output()),
        ))
    }
}

impl GrpcServiceStub {
    /// Create an unbound stub for a service descriptor.
    pub(crate) fn create(service: ServiceDescriptor) -> Gd<Self> {
        Gd::from_init_fn(|base| GrpcServiceStub {
            service,
            channel: None,
            base,
        })
    }

    /// Resolve the method descriptor and the bound channel, logging on misuse.
    fn prepare(&self, method: &str) -> Option<(MethodDescriptor, Channel)> {
        let Some(channel) = self.channel.as_ref() else {
            godot_error!("[godot-grpc] stub not bound to a channel; call client(channel) first");
            return None;
        };
        let Some(m) = self.service.methods().find(|m| m.name() == method) else {
            godot_error!(
                "[godot-grpc] method {method:?} not found on {}",
                self.service.full_name()
            );
            return None;
        };
        Some((m, channel.bind().channel()))
    }

    fn encode_request(&self, m: &MethodDescriptor, request: &VarDictionary) -> Option<Vec<u8>> {
        match convert::dict_to_message(m.input(), request) {
            Ok(dm) => Some(dm.encode_to_vec()),
            Err(e) => {
                godot_error!("[godot-grpc] encoding request for {}: {e}", m.name());
                None
            }
        }
    }
}

fn method_path(service: &ServiceDescriptor, m: &MethodDescriptor) -> String {
    format!("/{}/{}", service.full_name(), m.name())
}
