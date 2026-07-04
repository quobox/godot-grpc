# Type mapping

In tier 2 (and generated code), protobuf types convert to and from Godot
`Variant` types as follows.

| Proto type | Godot type |
|---|---|
| `double`, `float` | `float` |
| `int32`, `sint32`, `sfixed32` | `int` |
| `int64`, `sint64`, `sfixed64` | `int` |
| `uint32`, `fixed32` | `int` |
| `uint64`, `fixed64` | `int` |
| `bool` | `bool` |
| `string` | `String` |
| `bytes` | `PackedByteArray` |
| `enum` | `int` (the enum value) |
| message | `GrpcMessage` |
| `repeated T` | `Array` |
| `map<K, V>` | `Dictionary` |

## Notes

- **64-bit unsigned integers**: values above `2^63 - 1` do not fit in Godot's
  signed 64-bit `int` and will wrap. Use `string` fields for very large numbers
  if exact representation matters.
- **Nested messages** are `GrpcMessage` instances. When *setting* a message
  field you may pass either a `GrpcMessage` or a plain `Dictionary`.
- **`GrpcMessage.to_dict()`** converts a whole message (recursively) to a
  `Dictionary`; **`get_field` / `set_field`** work field-by-field.
- Unset fields read back as their protobuf defaults (0, `""`, empty array, etc.).
