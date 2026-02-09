//! Safe wrapper around spandsp's G.711 codec (A-law and u-law).
//!
//! Provides both the stateful encoder/decoder (`G711State`) and stateless
//! sample-level conversion functions.

extern crate spandsp_sys;

use std::fmt;
use std::os::raw::c_int;
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};

/// G.711 encoding mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum G711Mode {
    /// ITU-T G.711 A-law.
    ALaw,
    /// ITU-T G.711 u-law (mu-law).
    ULaw,
}

impl G711Mode {
    fn as_raw(self) -> c_int {
        match self {
            G711Mode::ALaw => spandsp_sys::G711_ALAW as c_int,
            G711Mode::ULaw => spandsp_sys::G711_ULAW as c_int,
        }
    }
}

impl fmt::Display for G711Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            G711Mode::ALaw => f.write_str("A-law"),
            G711Mode::ULaw => f.write_str("u-law"),
        }
    }
}

/// RAII wrapper around `g711_state_t`.
///
/// Created via `G711State::new()`, which calls `g711_init(NULL, mode)`.
/// Freed on drop via `g711_free`.
pub struct G711State {
    ptr: NonNull<spandsp_sys::g711_state_t>,
    mode: G711Mode,
}

impl G711State {
    /// Create a new G.711 encoder/decoder state for the specified mode.
    pub fn new(mode: G711Mode) -> Result<Self> {
        let ptr = unsafe { spandsp_sys::g711_init(std::ptr::null_mut(), mode.as_raw()) };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { ptr, mode })
    }

    /// Returns the encoding mode this state was initialized with.
    pub fn mode(&self) -> G711Mode {
        self.mode
    }

    /// Encode linear PCM samples to G.711.
    ///
    /// Returns the number of G.711 bytes produced.
    pub fn encode(&mut self, g711_data: &mut [u8], amp: &[i16]) -> usize {
        let len = amp.len().min(c_int::MAX as usize) as c_int;
        unsafe {
            spandsp_sys::g711_encode(self.ptr.as_ptr(), g711_data.as_mut_ptr(), amp.as_ptr(), len)
                as usize
        }
    }

    /// Decode G.711 data to linear PCM samples.
    ///
    /// Returns the number of linear samples produced.
    pub fn decode(&mut self, amp: &mut [i16], g711_data: &[u8]) -> usize {
        let g711_bytes = g711_data.len().min(c_int::MAX as usize) as c_int;
        unsafe {
            spandsp_sys::g711_decode(
                self.ptr.as_ptr(),
                amp.as_mut_ptr(),
                g711_data.as_ptr(),
                g711_bytes,
            ) as usize
        }
    }

    /// Transcode between A-law and u-law (direction determined by the mode
    /// this state was initialised with).
    ///
    /// Returns the number of G.711 bytes produced.
    pub fn transcode(&mut self, g711_out: &mut [u8], g711_in: &[u8]) -> usize {
        let g711_bytes = g711_in.len().min(c_int::MAX as usize) as c_int;
        unsafe {
            spandsp_sys::g711_transcode(
                self.ptr.as_ptr(),
                g711_out.as_mut_ptr(),
                g711_in.as_ptr(),
                g711_bytes,
            ) as usize
        }
    }

    /// Return the raw pointer to the underlying state.
    pub fn as_ptr(&self) -> *mut spandsp_sys::g711_state_t {
        self.ptr.as_ptr()
    }
}

impl fmt::Debug for G711State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("G711State")
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}

impl Drop for G711State {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::g711_free(self.ptr.as_ptr());
        }
    }
}

// ---------------------------------------------------------------------------
// Stateless conversion functions
//
// These mirror the inline C functions from g711.h. Since bindgen may not
// generate bindings for static inline functions, we re-implement them in
// pure Rust. The algorithms are taken directly from the ITU G.711 spec.
// ---------------------------------------------------------------------------

/// Bias added during u-law encoding.
const ULAW_BIAS: i32 = 0x84;

/// A-law alternate mark inversion mask.
const ALAW_AMI_MASK: u8 = 0x55;

/// Find the position of the highest set bit (0-based from LSB).
/// Returns -1 when the input is 0.
#[inline]
fn top_bit(v: i32) -> i32 {
    if v == 0 {
        return -1;
    }
    // Use leading_zeros for efficiency; i32 is 32 bits.
    let v_unsigned = v as u32;
    (31 - v_unsigned.leading_zeros()) as i32
}

/// Encode a single linear PCM sample to u-law.
#[inline]
pub fn linear_to_ulaw(linear: i16) -> u8 {
    let mut lin = linear as i32;
    let mask: u8;
    if lin >= 0 {
        lin += ULAW_BIAS;
        mask = 0xFF;
    } else {
        lin = ULAW_BIAS - lin;
        mask = 0x7F;
    }
    let seg = top_bit(lin | 0xFF) - 7;
    if seg >= 8 {
        0x7F ^ mask
    } else {
        ((seg << 4) | ((lin >> (seg + 3)) & 0xF)) as u8 ^ mask
    }
}

/// Decode a single u-law sample to linear PCM.
#[inline]
pub fn ulaw_to_linear(ulaw: u8) -> i16 {
    let ulaw = !ulaw;
    let t = ((((ulaw & 0x0F) as i32) << 3) + ULAW_BIAS) << (((ulaw as i32) & 0x70) >> 4);
    if ulaw & 0x80 != 0 {
        (ULAW_BIAS - t) as i16
    } else {
        (t - ULAW_BIAS) as i16
    }
}

/// Encode a single linear PCM sample to A-law.
#[inline]
pub fn linear_to_alaw(linear: i16) -> u8 {
    let mut lin = linear as i32;
    let mask: u8;
    if lin >= 0 {
        mask = 0x80 | ALAW_AMI_MASK;
    } else {
        mask = ALAW_AMI_MASK;
        lin = -lin - 1;
    }
    let seg = top_bit(lin | 0xFF) - 7;
    if seg >= 8 {
        0x7F ^ mask
    } else {
        let shift = if seg != 0 { seg + 3 } else { 4 };
        ((seg << 4) | ((lin >> shift) & 0x0F)) as u8 ^ mask
    }
}

/// Decode a single A-law sample to linear PCM.
#[inline]
pub fn alaw_to_linear(alaw: u8) -> i16 {
    let alaw = alaw ^ ALAW_AMI_MASK;
    let i = ((alaw & 0x0F) as i32) << 4;
    let seg = ((alaw as i32) & 0x70) >> 4;
    let val = if seg != 0 {
        (i + 0x108) << (seg - 1)
    } else {
        i + 8
    };
    if alaw & 0x80 != 0 {
        val as i16
    } else {
        -(val as i16)
    }
}

/// Transcode a single A-law sample to u-law using the ITU-specified procedure.
#[inline]
pub fn alaw_to_ulaw(alaw: u8) -> u8 {
    unsafe { spandsp_sys::alaw_to_ulaw(alaw) }
}

/// Transcode a single u-law sample to A-law using the ITU-specified procedure.
#[inline]
pub fn ulaw_to_alaw(ulaw: u8) -> u8 {
    unsafe { spandsp_sys::ulaw_to_alaw(ulaw) }
}
