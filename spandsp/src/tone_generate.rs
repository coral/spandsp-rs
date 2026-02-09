//! Safe wrappers around spandsp's tone generation subsystem.
//!
//! - `ToneGenDescriptor` wraps `tone_gen_descriptor_t`.
//! - `ToneGenerator` wraps `tone_gen_state_t`.

extern crate spandsp_sys;

use std::fmt;
use std::os::raw::c_int;
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};

/// A frequency + level pair for tone generation.
///
/// - `frequency`: tone frequency in Hz. Use 0 for none, negative for AM modulation.
/// - `level`: signal level in dBm0 (or modulation depth % for AM).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToneFreq {
    pub frequency: i32,
    pub level: i32,
}

impl ToneFreq {
    /// Create a new tone component.
    pub const fn new(frequency: i32, level: i32) -> Self {
        Self { frequency, level }
    }

    /// No tone (frequency 0, level 0).
    pub const NONE: Self = Self {
        frequency: 0,
        level: 0,
    };
}

impl Default for ToneFreq {
    fn default() -> Self {
        Self::NONE
    }
}

impl fmt::Display for ToneFreq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Hz @ {} dBm0", self.frequency, self.level)
    }
}

/// On/off cadence timing for tone generation.
///
/// Durations are in milliseconds. Use 0 for unused segments.
///
/// A typical pattern is `on1` / `off1` for a simple repeating cadence,
/// with `on2` / `off2` for more complex patterns (e.g. distinctive ring).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToneCadence {
    pub on1: i32,
    pub off1: i32,
    pub on2: i32,
    pub off2: i32,
}

impl ToneCadence {
    /// Create a cadence from four durations in milliseconds.
    pub const fn new(on1: i32, off1: i32, on2: i32, off2: i32) -> Self {
        Self {
            on1,
            off1,
            on2,
            off2,
        }
    }

    /// Simple on/off cadence (two-segment).
    pub const fn simple(on: i32, off: i32) -> Self {
        Self {
            on1: on,
            off1: off,
            on2: 0,
            off2: 0,
        }
    }

    /// Continuous tone (no cadence, single on period).
    pub const fn continuous(duration: i32) -> Self {
        Self {
            on1: duration,
            off1: 0,
            on2: 0,
            off2: 0,
        }
    }
}

impl fmt::Display for ToneCadence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.on2 == 0 && self.off2 == 0 {
            write!(f, "{}ms on / {}ms off", self.on1, self.off1)
        } else {
            write!(
                f,
                "{}ms on / {}ms off / {}ms on / {}ms off",
                self.on1, self.off1, self.on2, self.off2
            )
        }
    }
}

/// Descriptor for a cadenced multi-tone generator.
///
/// Created via `ToneGenDescriptor::new()`, which calls
/// `tone_gen_descriptor_init(NULL, ...)` to let spandsp allocate. Freed via
/// `tone_gen_descriptor_free` on drop.
pub struct ToneGenDescriptor {
    ptr: NonNull<spandsp_sys::tone_gen_descriptor_t>,
}

impl ToneGenDescriptor {
    /// Create a new tone generator descriptor.
    ///
    /// # Parameters
    ///
    /// - `tone1`: first tone component (frequency + level).
    /// - `tone2`: second tone component, or `ToneFreq::NONE` for single-tone.
    /// - `cadence`: on/off timing pattern.
    /// - `repeat`: if `true`, the cadence repeats.
    pub fn new(
        tone1: ToneFreq,
        tone2: ToneFreq,
        cadence: ToneCadence,
        repeat: bool,
    ) -> Result<Self> {
        let ptr = unsafe {
            spandsp_sys::tone_gen_descriptor_init(
                std::ptr::null_mut(),
                tone1.frequency as c_int,
                tone1.level as c_int,
                tone2.frequency as c_int,
                tone2.level as c_int,
                cadence.on1 as c_int,
                cadence.off1 as c_int,
                cadence.on2 as c_int,
                cadence.off2 as c_int,
                repeat as c_int,
            )
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { ptr })
    }

    /// Return the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::tone_gen_descriptor_t {
        self.ptr.as_ptr()
    }
}

impl Drop for ToneGenDescriptor {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::tone_gen_descriptor_free(self.ptr.as_ptr());
        }
    }
}

/// Cadenced multi-tone generator state.
///
/// Created from a `ToneGenDescriptor`. Freed via `tone_gen_free` on drop.
pub struct ToneGenerator {
    ptr: NonNull<spandsp_sys::tone_gen_state_t>,
}

impl ToneGenerator {
    /// Create a new tone generator from a descriptor.
    pub fn new(descriptor: &ToneGenDescriptor) -> Result<Self> {
        let ptr = unsafe { spandsp_sys::tone_gen_init(std::ptr::null_mut(), descriptor.as_ptr()) };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { ptr })
    }

    /// Generate tone samples.
    ///
    /// Returns the number of samples actually generated. A return value of 0
    /// indicates the tone cadence has completed.
    pub fn generate(&mut self, amp: &mut [i16]) -> usize {
        let max_samples = amp.len().min(c_int::MAX as usize) as c_int;
        unsafe { spandsp_sys::tone_gen(self.ptr.as_ptr(), amp.as_mut_ptr(), max_samples) as usize }
    }

    /// Return the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::tone_gen_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for ToneGenerator {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::tone_gen_free(self.ptr.as_ptr());
        }
    }
}
