//! Safe wrappers around spandsp's G.722 wideband codec.
//!
//! - `G722Encoder` wraps `g722_encode_state_t`.
//! - `G722Decoder` wraps `g722_decode_state_t`.

extern crate spandsp_sys;

use std::fmt;
use std::os::raw::c_int;
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};

bitflags::bitflags! {
    /// G.722 codec option flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct G722Options: i32 {
        /// Operate at 8000 samples/second (narrowband interworking mode).
        const SAMPLE_RATE_8000 = 0x0001;
        /// Use packed bit ordering.
        const PACKED = 0x0002;
    }
}

impl Default for G722Options {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for G722Options {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

/// Valid bit rates for G.722.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum G722Rate {
    /// 64000 bits/s (mode 1).
    Rate64000,
    /// 56000 bits/s (mode 2).
    Rate56000,
    /// 48000 bits/s (mode 3).
    Rate48000,
}

impl G722Rate {
    fn as_raw(self) -> c_int {
        match self {
            G722Rate::Rate64000 => 64000,
            G722Rate::Rate56000 => 56000,
            G722Rate::Rate48000 => 48000,
        }
    }

    /// Returns the bit rate in bits per second.
    pub fn bps(self) -> u32 {
        self.as_raw() as u32
    }
}

impl fmt::Display for G722Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            G722Rate::Rate64000 => f.write_str("64 kbit/s"),
            G722Rate::Rate56000 => f.write_str("56 kbit/s"),
            G722Rate::Rate48000 => f.write_str("48 kbit/s"),
        }
    }
}

impl TryFrom<u32> for G722Rate {
    type Error = SpanDspError;

    fn try_from(bps: u32) -> std::result::Result<Self, Self::Error> {
        match bps {
            64000 => Ok(G722Rate::Rate64000),
            56000 => Ok(G722Rate::Rate56000),
            48000 => Ok(G722Rate::Rate48000),
            _ => Err(SpanDspError::InvalidInput(format!(
                "invalid G.722 rate: {bps} bps"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Encoder
// ---------------------------------------------------------------------------

/// RAII wrapper around `g722_encode_state_t`.
///
/// Created via `G722Encoder::new()`. Freed on drop via `g722_encode_free`.
pub struct G722Encoder {
    ptr: NonNull<spandsp_sys::g722_encode_state_t>,
}

impl G722Encoder {
    /// Create a new G.722 encoder.
    pub fn new(rate: G722Rate, options: G722Options) -> Result<Self> {
        let ptr = unsafe {
            spandsp_sys::g722_encode_init(
                std::ptr::null_mut(),
                rate.as_raw(),
                options.bits() as c_int,
            )
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { ptr })
    }

    /// Encode linear PCM audio to G.722.
    ///
    /// Returns the number of G.722 bytes produced.
    pub fn encode(&mut self, g722_data: &mut [u8], amp: &[i16]) -> usize {
        let len = amp.len().min(c_int::MAX as usize) as c_int;
        unsafe {
            spandsp_sys::g722_encode(self.ptr.as_ptr(), g722_data.as_mut_ptr(), amp.as_ptr(), len)
                as usize
        }
    }

    /// Return the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::g722_encode_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for G722Encoder {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::g722_encode_free(self.ptr.as_ptr());
        }
    }
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

/// RAII wrapper around `g722_decode_state_t`.
///
/// Created via `G722Decoder::new()`. Freed on drop via `g722_decode_free`.
pub struct G722Decoder {
    ptr: NonNull<spandsp_sys::g722_decode_state_t>,
}

impl G722Decoder {
    /// Create a new G.722 decoder.
    pub fn new(rate: G722Rate, options: G722Options) -> Result<Self> {
        let ptr = unsafe {
            spandsp_sys::g722_decode_init(
                std::ptr::null_mut(),
                rate.as_raw(),
                options.bits() as c_int,
            )
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { ptr })
    }

    /// Decode G.722 data to linear PCM.
    ///
    /// Returns the number of PCM samples produced.
    pub fn decode(&mut self, amp: &mut [i16], g722_data: &[u8]) -> usize {
        let len = g722_data.len().min(c_int::MAX as usize) as c_int;
        unsafe {
            spandsp_sys::g722_decode(self.ptr.as_ptr(), amp.as_mut_ptr(), g722_data.as_ptr(), len)
                as usize
        }
    }

    /// Return the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::g722_decode_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for G722Decoder {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::g722_decode_free(self.ptr.as_ptr());
        }
    }
}
