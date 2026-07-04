//! `GrpcMessage`: a tier-2 `RefCounted` wrapper around a
//! `prost_reflect::DynamicMessage`, exposing field access and `Dictionary`
//! conversion to GDScript.
//!
//! It is GDScript-constructible (`#[class(init)]`) so generated typed classes
//! can `extend GrpcMessage` and bind their proto type in `_init` via
//! [`bind_type`][GrpcMessage::bind_type]. Until bound (or wrapped via
//! [`from_dynamic`]), the inner message is `None` and accessors are no-ops.

use godot::prelude::*;
use prost_reflect::{DynamicMessage, ReflectMessage};

use crate::convert;
use crate::descriptor::GrpcDescriptorPool;

#[derive(GodotClass)]
#[class(init, base = RefCounted)]
pub struct GrpcMessage {
    msg: Option<DynamicMessage>,
    base: Base<RefCounted>,
}

#[godot_api]
impl GrpcMessage {
    /// Bind this message to a proto type from `pool` (called by generated
    /// `_init`). Replaces any current contents with an empty message of that
    /// type. Returns false (and logs) if the type is unknown.
    #[func]
    fn bind_type(&mut self, pool: Gd<GrpcDescriptorPool>, type_name: GString) -> bool {
        match pool.bind().message_descriptor(&type_name.to_string()) {
            Some(desc) => {
                self.msg = Some(DynamicMessage::new(desc));
                true
            }
            None => {
                godot_error!("[godot-grpc] type {type_name:?} not found in descriptor pool");
                false
            }
        }
    }

    /// Copy another message's contents into this one (used by generated
    /// `wrap()` to give a typed view of a response).
    #[func]
    fn adopt(&mut self, other: Gd<GrpcMessage>) {
        self.msg = other.bind().msg.clone();
    }

    /// Get a field by name as a `Variant`. Scalars read back their
    /// proto default when unset; an unset singular **message** field reads back
    /// `null` (so `if x == null` presence checks work); unknown fields and an
    /// unbound message also yield `null`.
    #[func]
    fn get_field(&self, name: GString) -> Variant {
        let Some(msg) = self.msg.as_ref() else {
            return Variant::nil();
        };
        let name = name.to_string();
        if let Some(field) = msg.descriptor().get_field_by_name(&name) {
            // Singular message field uses presence: report unset as null rather
            // than an empty default message.
            if !field.is_list()
                && !field.is_map()
                && matches!(field.kind(), prost_reflect::Kind::Message(_))
                && !msg.has_field_by_name(&name)
            {
                return Variant::nil();
            }
        }
        match msg.get_field_by_name(&name) {
            Some(value) => convert::value_to_variant(&value),
            None => Variant::nil(),
        }
    }

    /// Set a field by name from a `Variant`. Logs on an unknown field, wrong
    /// type, or unbound message.
    #[func]
    fn set_field(&mut self, name: GString, value: Variant) {
        let Some(msg) = self.msg.as_mut() else {
            godot_error!("[godot-grpc] set_field on an unbound GrpcMessage");
            return;
        };
        let name = name.to_string();
        let Some(field) = msg.descriptor().get_field_by_name(&name) else {
            godot_error!(
                "[godot-grpc] no field {name:?} on {}",
                msg.descriptor().full_name()
            );
            return;
        };
        match convert::variant_to_field_value(&field, &value) {
            Ok(v) => {
                if let Err(e) = msg.try_set_field_by_name(&name, v) {
                    godot_error!("[godot-grpc] set_field {name:?}: {e:?}");
                }
            }
            Err(e) => godot_error!("[godot-grpc] set_field {name:?}: {e}"),
        }
    }

    /// Whether the field with this name is set.
    #[func]
    fn has_field(&self, name: GString) -> bool {
        self.msg
            .as_ref()
            .is_some_and(|m| m.has_field_by_name(&name.to_string()))
    }

    /// The whole message as a `Dictionary` keyed by field name (empty if unbound).
    #[func]
    fn to_dict(&self) -> VarDictionary {
        self.msg
            .as_ref()
            .map(convert::message_to_dict)
            .unwrap_or_default()
    }

    /// The fully-qualified message type name, or `""` if unbound.
    #[func]
    fn type_name(&self) -> GString {
        self.msg
            .as_ref()
            .map(|m| GString::from(m.descriptor().full_name()))
            .unwrap_or_default()
    }
}

impl GrpcMessage {
    /// Wrap a decoded `DynamicMessage` (tier-2 responses).
    pub(crate) fn from_dynamic(msg: DynamicMessage) -> Gd<Self> {
        Gd::from_init_fn(|base| GrpcMessage {
            msg: Some(msg),
            base,
        })
    }

    /// Borrow the underlying dynamic message, if bound.
    pub(crate) fn dynamic(&self) -> Option<&DynamicMessage> {
        self.msg.as_ref()
    }
}
