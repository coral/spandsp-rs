//! Safe wrapper around spandsp's G.726 ADPCM codec.
//!
//! Wraps `g726_state_t` for both encoding and decoding.

extern crate spandsp_sys;

use std::fmt;
use std::os::raw::c_int;
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};

/// External coding type for G.726 interworking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum G726Encoding {
    /// Interworking with 16-bit signed linear PCM.
    Linear,
    /// Interworking with u-law.
    ULaw,
    /// Interworking with A-law.
    ALaw,
}

impl G726Encoding {
    fn as_raw(self) -> c_int {
        match self {
            G726Encoding::Linear => spandsp_sys::G726_ENCODING_LINEAR as c_int,
            G726Encoding::ULaw => spandsp_sys::G726_ENCODING_ULAW as c_int,
            G726Encoding::ALaw => spandsp_sys::G726_ENCODING_ALAW as c_int,
        }
    }
}

impl fmt::Display for G726Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            G726Encoding::Linear => f.write_str("linear"),
            G726Encoding::ULaw => f.write_str("u-law"),
            G726Encoding::ALaw => f.write_str("A-law"),
        }
    }
}

/// G.726 bit packing mode.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum G726Packing {
    /// No packing.
    #[default]
    None,
    /// Left-justified packing.
    Left,
    /// Right-justified packing.
    Right,
}

impl G726Packing {
    fn as_raw(self) -> c_int {
        match self {
            G726Packing::None => spandsp_sys::G726_PACKING_NONE as c_int,
            G726Packing::Left => spandsp_sys::G726_PACKING_LEFT as c_int,
            G726Packing::Right => spandsp_sys::G726_PACKING_RIGHT as c_int,
        }
    }
}

impl fmt::Display for G726Packing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            G726Packing::None => f.write_str("none"),
            G726Packing::Left => f.write_str("left"),
            G726Packing::Right => f.write_str("right"),
        }
    }
}

/// Valid bit rates for G.726.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum G726Rate {
    /// 16 kbit/s (2 bits per sample).
    Rate16000,
    /// 24 kbit/s (3 bits per sample).
    Rate24000,
    /// 32 kbit/s (4 bits per sample).
    Rate32000,
    /// 40 kbit/s (5 bits per sample).
    Rate40000,
}

impl G726Rate {
    fn as_raw(self) -> c_int {
        match self {
            G726Rate::Rate16000 => 16000,
            G726Rate::Rate24000 => 24000,
            G726Rate::Rate32000 => 32000,
            G726Rate::Rate40000 => 40000,
        }
    }

    /// Returns the bit rate in bits per second.
    pub fn bps(self) -> u32 {
        self.as_raw() as u32
    }

    /// Returns the number of bits per ADPCM sample.
    pub fn bits_per_sample(self) -> u8 {
        match self {
            G726Rate::Rate16000 => 2,
            G726Rate::Rate24000 => 3,
            G726Rate::Rate32000 => 4,
            G726Rate::Rate40000 => 5,
        }
    }
}

impl fmt::Display for G726Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            G726Rate::Rate16000 => f.write_str("16 kbit/s"),
            G726Rate::Rate24000 => f.write_str("24 kbit/s"),
            G726Rate::Rate32000 => f.write_str("32 kbit/s"),
            G726Rate::Rate40000 => f.write_str("40 kbit/s"),
        }
    }
}

impl TryFrom<u32> for G726Rate {
    type Error = SpanDspError;

    fn try_from(bps: u32) -> std::result::Result<Self, Self::Error> {
        match bps {
            16000 => Ok(G726Rate::Rate16000),
            24000 => Ok(G726Rate::Rate24000),
            32000 => Ok(G726Rate::Rate32000),
            40000 => Ok(G726Rate::Rate40000),
            _ => Err(SpanDspError::InvalidInput(format!(
                "invalid G.726 rate: {bps} bps"
            ))),
        }
    }
}

/// RAII wrapper around `g726_state_t`.
///
/// A single state handles both encoding and decoding, depending on which
/// method is called. Created via `G726State::new()`. Freed on drop via
/// `g726_free`.
pub struct G726State {
    ptr: NonNull<spandsp_sys::g726_state_t>,
}

impl G726State {
    /// Create a new G.726 state.
    pub fn new(rate: G726Rate, encoding: G726Encoding, packing: G726Packing) -> Result<Self> {
        let ptr = unsafe {
            spandsp_sys::g726_init(
                std::ptr::null_mut(),
                rate.as_raw(),
                encoding.as_raw(),
                packing.as_raw(),
            )
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { ptr })
    }

    /// Encode linear PCM (or A-law/u-law per init) to G.726.
    ///
    /// Returns the number of G.726 bytes produced.
    pub fn encode(&mut self, g726_data: &mut [u8], amp: &[i16]) -> usize {
        let len = amp.len().min(c_int::MAX as usize) as c_int;
        unsafe {
            spandsp_sys::g726_encode(self.ptr.as_ptr(), g726_data.as_mut_ptr(), amp.as_ptr(), len)
                as usize
        }
    }

    /// Decode G.726 data to linear PCM (or A-law/u-law per init).
    ///
    /// Returns the number of samples produced.
    pub fn decode(&mut self, amp: &mut [i16], g726_data: &[u8]) -> usize {
        let g726_bytes = g726_data.len().min(c_int::MAX as usize) as c_int;
        unsafe {
            spandsp_sys::g726_decode(
                self.ptr.as_ptr(),
                amp.as_mut_ptr(),
                g726_data.as_ptr(),
                g726_bytes,
            ) as usize
        }
    }

    /// Return the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::g726_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for G726State {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::g726_free(self.ptr.as_ptr());
        }
    }
}
