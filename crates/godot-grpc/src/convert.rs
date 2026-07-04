//! Conversion between Godot `Variant`s and `prost_reflect` dynamic values,
//! used by tier-2 `GrpcMessage` and `GrpcServiceStub`.
//!
//! Runs entirely on the Godot main thread (it touches `Variant`s), never on the
//! Tokio thread.

use std::collections::HashMap;

use godot::prelude::*;
use prost_reflect::{
    DynamicMessage, FieldDescriptor, Kind, MapKey, MessageDescriptor, ReflectMessage, Value,
};

use crate::message::GrpcMessage;

// ---------------------------------------------------------------------------
// prost_reflect::Value -> Variant
// ---------------------------------------------------------------------------

/// Convert a dynamic protobuf value to a Godot `Variant`.
pub(crate) fn value_to_variant(value: &Value) -> Variant {
    match value {
        Value::Bool(b) => b.to_variant(),
        Value::I32(i) => (*i as i64).to_variant(),
        Value::I64(i) => i.to_variant(),
        Value::U32(u) => (*u as i64).to_variant(),
        Value::U64(u) => (*u as i64).to_variant(),
        Value::F32(f) => (*f as f64).to_variant(),
        Value::F64(f) => f.to_variant(),
        Value::String(s) => GString::from(s.as_str()).to_variant(),
        Value::Bytes(b) => PackedByteArray::from(b.as_ref()).to_variant(),
        Value::EnumNumber(i) => (*i as i64).to_variant(),
        Value::Message(m) => GrpcMessage::from_dynamic(m.clone()).to_variant(),
        Value::List(items) => {
            let mut arr = VarArray::new();
            for it in items {
                arr.push(&value_to_variant(it));
            }
            arr.to_variant()
        }
        Value::Map(map) => {
            let mut dict = VarDictionary::new();
            for (k, v) in map {
                dict.set(&map_key_to_variant(k), &value_to_variant(v));
            }
            dict.to_variant()
        }
    }
}

fn map_key_to_variant(k: &MapKey) -> Variant {
    match k {
        MapKey::Bool(b) => b.to_variant(),
        MapKey::I32(i) => (*i as i64).to_variant(),
        MapKey::I64(i) => i.to_variant(),
        MapKey::U32(u) => (*u as i64).to_variant(),
        MapKey::U64(u) => (*u as i64).to_variant(),
        MapKey::String(s) => GString::from(s.as_str()).to_variant(),
    }
}

/// Convert a whole message into a `Dictionary` keyed by field name.
pub(crate) fn message_to_dict(msg: &DynamicMessage) -> VarDictionary {
    let mut dict = VarDictionary::new();
    for field in msg.descriptor().fields() {
        let value = msg.get_field(&field);
        dict.set(&field.name().to_variant(), &value_to_variant(&value));
    }
    dict
}

// ---------------------------------------------------------------------------
// Variant -> prost_reflect::Value
// ---------------------------------------------------------------------------

/// Build a `DynamicMessage` of type `desc` from a `Dictionary` (field name -> value).
/// Absent fields keep their protobuf default.
pub(crate) fn dict_to_message(
    desc: MessageDescriptor,
    dict: &VarDictionary,
) -> Result<DynamicMessage, String> {
    let mut msg = DynamicMessage::new(desc.clone());
    for field in desc.fields() {
        if let Some(variant) = dict.get(&field.name().to_variant()) {
            // An explicit `null` in the dict means "leave default" (unset), not
            // a type error.
            if variant.is_nil() {
                continue;
            }
            let value = variant_to_field_value(&field, &variant)?;
            msg.try_set_field_by_name(field.name(), value)
                .map_err(|e| format!("field {:?}: {e:?}", field.name()))?;
        }
    }
    Ok(msg)
}

/// Convert a `Variant` to a `Value` for a specific field, honouring repeated/map
/// cardinality before falling back to the scalar/message element conversion.
pub(crate) fn variant_to_field_value(
    field: &FieldDescriptor,
    v: &Variant,
) -> Result<Value, String> {
    if field.is_map() {
        let dict = v
            .try_to::<VarDictionary>()
            .map_err(|_| format!("field {:?} expects a Dictionary (map)", field.name()))?;
        let Kind::Message(entry) = field.kind() else {
            return Err(format!(
                "map field {:?} is not a message entry",
                field.name()
            ));
        };
        let key_kind = entry.map_entry_key_field().kind();
        let value_kind = entry.map_entry_value_field().kind();
        let mut map = HashMap::new();
        for (k, val) in dict.iter_shared() {
            map.insert(
                variant_to_map_key(&key_kind, &k)?,
                scalar_to_value(&value_kind, &val)?,
            );
        }
        Ok(Value::Map(map))
    } else if field.is_list() {
        let arr = v
            .try_to::<VarArray>()
            .map_err(|_| format!("field {:?} expects an Array (repeated)", field.name()))?;
        let kind = field.kind();
        let mut list = Vec::with_capacity(arr.len());
        for it in arr.iter_shared() {
            list.push(scalar_to_value(&kind, &it)?);
        }
        Ok(Value::List(list))
    } else {
        scalar_to_value(&field.kind(), v)
    }
}

fn scalar_to_value(kind: &Kind, v: &Variant) -> Result<Value, String> {
    Ok(match kind {
        Kind::Double => Value::F64(to_f64(v)?),
        Kind::Float => Value::F32(to_f64(v)? as f32),
        Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => Value::I32(narrow_i32(to_i64(v)?)?),
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => Value::I64(to_i64(v)?),
        Kind::Uint32 | Kind::Fixed32 => Value::U32(narrow_u32(to_i64(v)?)?),
        // Godot int is i64, so a uint64 above 2^63 arrives as a negative int;
        // `as u64` restores the exact wire value via two's complement.
        Kind::Uint64 | Kind::Fixed64 => Value::U64(to_i64(v)? as u64),
        Kind::Bool => Value::Bool(
            v.try_to::<bool>()
                .map_err(|_| "expected bool".to_string())?,
        ),
        Kind::String => Value::String(
            v.try_to::<GString>()
                .map_err(|_| "expected String".to_string())?
                .to_string(),
        ),
        Kind::Bytes => Value::Bytes(bytes::Bytes::from(
            v.try_to::<PackedByteArray>()
                .map_err(|_| "expected PackedByteArray".to_string())?
                .to_vec(),
        )),
        Kind::Enum(_) => Value::EnumNumber(narrow_i32(to_i64(v)?)?),
        Kind::Message(md) => Value::Message(variant_to_message(md, v)?),
    })
}

/// Range-check an `i64` into a 32-bit signed proto field (no silent wrap).
fn narrow_i32(v: i64) -> Result<i32, String> {
    i32::try_from(v).map_err(|_| format!("value {v} out of range for a 32-bit signed field"))
}

/// Range-check an `i64` into a 32-bit unsigned proto field (no silent wrap).
fn narrow_u32(v: i64) -> Result<u32, String> {
    u32::try_from(v).map_err(|_| format!("value {v} out of range for a 32-bit unsigned field"))
}

fn variant_to_message(md: &MessageDescriptor, v: &Variant) -> Result<DynamicMessage, String> {
    if let Ok(gd) = v.try_to::<Gd<GrpcMessage>>() {
        match gd.bind().dynamic() {
            Some(dm) => Ok(dm.clone()),
            None => Err("GrpcMessage has no bound type".to_string()),
        }
    } else if let Ok(dict) = v.try_to::<VarDictionary>() {
        dict_to_message(md.clone(), &dict)
    } else {
        Err(format!(
            "field expects a Dictionary or GrpcMessage ({})",
            md.full_name()
        ))
    }
}

fn variant_to_map_key(kind: &Kind, v: &Variant) -> Result<MapKey, String> {
    Ok(match kind {
        Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => MapKey::I32(narrow_i32(to_i64(v)?)?),
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => MapKey::I64(to_i64(v)?),
        Kind::Uint32 | Kind::Fixed32 => MapKey::U32(narrow_u32(to_i64(v)?)?),
        Kind::Uint64 | Kind::Fixed64 => MapKey::U64(to_i64(v)? as u64),
        Kind::Bool => MapKey::Bool(
            v.try_to::<bool>()
                .map_err(|_| "expected bool key".to_string())?,
        ),
        Kind::String => MapKey::String(
            v.try_to::<GString>()
                .map_err(|_| "expected String key".to_string())?
                .to_string(),
        ),
        _ => return Err("invalid map key type".to_string()),
    })
}

/// Accept an int Variant, or a float that is exactly integral (so `3.0` is fine
/// but `3.9` is an error rather than a silent truncation to `3`).
fn to_i64(v: &Variant) -> Result<i64, String> {
    if let Ok(i) = v.try_to::<i64>() {
        return Ok(i);
    }
    if let Ok(f) = v.try_to::<f64>() {
        if f.is_finite() && f.fract() == 0.0 && (i64::MIN as f64..=i64::MAX as f64).contains(&f) {
            return Ok(f as i64);
        }
        return Err(format!("expected an integer, got {f}"));
    }
    Err("expected a number".to_string())
}

/// Accept either a float or an int Variant as a float.
fn to_f64(v: &Variant) -> Result<f64, String> {
    v.try_to::<f64>()
        .or_else(|_| v.try_to::<i64>().map(|i| i as f64))
        .map_err(|_| "expected a number".to_string())
}
