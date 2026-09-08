//! Raw SpanDSP bindings. V.150.1, SPRT, and SSE APIs require the opt-in `v150`
//! feature, which compiles native code carrying GPL-2.0-only notices.
//! The feature is disabled by default; vendored source packaging is unchanged.

#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
