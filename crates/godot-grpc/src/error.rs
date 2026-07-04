//! gRPC error model exposed to GDScript: [`GrpcStatus`] and [`GrpcError`].
//!
//! These are `RefCounted` data objects produced by the library (never `.new()`d
//! from GDScript), carrying the outcome of an RPC. Async RPC failures are
//! delivered through the `failed(GrpcStatus)` signal on `GrpcCall` (see spec
//! §6.2); `GrpcError` additionally carries a Rust-side cause chain for
//! synchronous fallible operations.

use godot::prelude::*;
use tonic::Status;

/// A gRPC status: numeric code, message, and a human-readable description.
/// Mirrors `tonic::Status` / the standard gRPC status codes (0 = OK .. 16).
#[derive(GodotClass)]
#[class(no_init, base = RefCounted)]
pub struct GrpcStatus {
    code: i64,
    message: GString,
    base: Base<RefCounted>,
}

#[godot_api]
impl GrpcStatus {
    /// The numeric gRPC status code (0 = OK, 1 = CANCELLED, ... 16 = UNAUTHENTICATED).
    #[func]
    pub fn code(&self) -> i64 {
        self.code
    }

    /// The status message returned by the server (may be empty).
    #[func]
    pub fn message(&self) -> GString {
        self.message.clone()
    }

    /// A static human-readable description of the status code.
    #[func]
    pub fn description(&self) -> GString {
        tonic::Code::from(self.code as i32).description().into()
    }

    /// Whether this status represents success (code == 0).
    #[func]
    pub fn is_ok(&self) -> bool {
        self.code == 0
    }
}

// Constructors are wired up in M4 (channel) / M5 (call).
#[allow(dead_code)]
impl GrpcStatus {
    /// Build a `GrpcStatus` object from a `tonic::Status`.
    pub(crate) fn from_tonic(status: &Status) -> Gd<Self> {
        Self::create(i32::from(status.code()) as i64, status.message())
    }

    /// Build a `GrpcStatus` object from a raw code and message.
    pub(crate) fn create(code: i64, message: &str) -> Gd<Self> {
        let message = GString::from(message);
        Gd::from_init_fn(|base| GrpcStatus {
            code,
            message,
            base,
        })
    }
}

/// An error from a gRPC operation: a [`GrpcStatus`] plus a Rust-side cause chain
/// serialized to a string (transport/connection failures, etc.).
#[derive(GodotClass)]
#[class(no_init, base = RefCounted)]
pub struct GrpcError {
    status: Gd<GrpcStatus>,
    cause: GString,
    base: Base<RefCounted>,
}

#[godot_api]
impl GrpcError {
    /// The gRPC status underlying this error.
    #[func]
    pub fn status(&self) -> Gd<GrpcStatus> {
        self.status.clone()
    }

    /// The status message, for convenience (`status().message()`).
    #[func]
    pub fn message(&self) -> GString {
        self.status.bind().message()
    }

    /// The full Rust-side cause chain (empty if none).
    #[func]
    pub fn cause(&self) -> GString {
        self.cause.clone()
    }
}

// Constructors are wired up in M5 (call).
#[allow(dead_code)]
impl GrpcError {
    /// Build a `GrpcError` from a `tonic::Status`, capturing its source chain.
    pub(crate) fn from_tonic(status: &Status) -> Gd<Self> {
        let mut cause = String::new();
        let mut src = std::error::Error::source(status);
        while let Some(e) = src {
            if !cause.is_empty() {
                cause.push_str(": ");
            }
            cause.push_str(&e.to_string());
            src = e.source();
        }
        let status = GrpcStatus::from_tonic(status);
        Gd::from_init_fn(|base| GrpcError {
            status,
            cause: GString::from(&cause),
            base,
        })
    }

    /// Build a `GrpcError` from a transport/connection-level error (no gRPC status).
    pub(crate) fn from_transport(code: tonic::Code, err: impl std::fmt::Display) -> Gd<Self> {
        let msg = err.to_string();
        let status = GrpcStatus::create(i32::from(code) as i64, &msg);
        Gd::from_init_fn(|base| GrpcError {
            status,
            cause: GString::from(&msg),
            base,
        })
    }
}
