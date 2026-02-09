//! Safe, idiomatic Rust wrappers for the [spandsp](https://github.com/freeswitch/spandsp)
//! telephony DSP library.
//!
//! Provides RAII-managed types for codecs (G.711, G.722, G.726), DTMF
//! generation/detection, HDLC framing, tone generation, Goertzel detection,
//! echo cancellation, power metering, and (with the `fax` feature) full
//! T.30/T.38/T.4 fax support.

pub mod error;
pub mod logging;

pub mod dtmf;
pub mod echo;
pub mod g711;
pub mod g722;
pub mod g726;
pub mod hdlc;
pub mod power_meter;
pub mod tone_detect;
pub mod tone_generate;

#[cfg(feature = "fax")]
pub mod fax;
#[cfg(feature = "fax")]
pub mod fax_modems;
#[cfg(feature = "fax")]
pub mod t30;
#[cfg(feature = "fax")]
pub mod t38_core;
#[cfg(feature = "fax")]
pub mod t38_gateway;
#[cfg(feature = "fax")]
pub mod t38_terminal;
#[cfg(feature = "fax")]
pub mod t4_rx;
#[cfg(feature = "fax")]
pub mod t4_tx;
