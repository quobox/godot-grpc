# Streaming

All four gRPC modes are supported. Responses always arrive on the Godot main
thread via signals on the returned `GrpcCall`.

The examples below use the tier-2 stub; tier 1 has the equivalent
`server_stream_call` / `client_stream_call` / `bidi_call` on `GrpcChannel`.

## Server streaming

One request, many responses. Each message is a `stream_item`; `completed` marks
the end of the stream.

```gdscript
var call := stub.server_stream("ListItems", { "page": 1 })
call.stream_item.connect(func(item): print(item.get_field("name")))
call.completed.connect(func(_end): print("done"))
call.failed.connect(func(status): push_error(status.message()))
```

## Client streaming

Many requests, one response. Send with `send_dict` (tier 2) or `send` (tier 1,
raw bytes), then `close_send` to finish; the reply arrives as `completed`.

```gdscript
# `chunks` is your own data to upload, e.g. an Array of PackedByteArray.
var call := stub.client_stream("UploadChunks")
call.completed.connect(func(reply): print(reply.get_field("status")))
for chunk in chunks:
    call.send_dict({ "data": chunk })
call.close_send()
```

## Bidirectional streaming

Send and receive independently.

```gdscript
var call := stub.bidi("Chat")
call.stream_item.connect(func(msg): print(msg.get_field("text")))
call.completed.connect(func(_end): print("closed"))
call.send_dict({ "text": "hello" })
# ... later ...
call.close_send()
```

## Concurrency

A single `GrpcChannel` multiplexes concurrent RPCs — you can have many calls in
flight on one channel. Each call is independent and delivers to its own signals.
