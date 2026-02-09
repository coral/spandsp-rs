//! Safe wrapper around spandsp's voice echo canceller.
//!
//! Wraps `echo_can_state_t` for G.168-style line echo cancellation.

extern crate spandsp_sys;

use std::fmt;
use std::os::raw::c_int;
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};

bitflags::bitflags! {
    /// Adaption mode flags for the echo canceller.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct EchoCanFlags: i32 {
        /// Enable adaption of the filter coefficients.
        const ADAPTION = 0x01;
        /// Enable non-linear processing (NLP) to suppress residual echo.
        const NLP = 0x02;
        /// Enable comfort noise generation (CNG).
        const CNG = 0x04;
        /// Enable clipping of residual echo.
        const CLIP = 0x08;
        /// Enable echo suppression.
        const SUPPRESSOR = 0x10;
        /// Enable transmit high-pass filter.
        const TX_HPF = 0x20;
        /// Enable receive high-pass filter.
        const RX_HPF = 0x40;
        /// Disable the echo canceller entirely.
        const DISABLE = 0x80;
    }
}

impl Default for EchoCanFlags {
    /// Default mode: adaption + NLP enabled.
    fn default() -> Self {
        Self::ADAPTION | Self::NLP
    }
}

impl fmt::Display for EchoCanFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

/// RAII wrapper around `echo_can_state_t`.
///
/// Note: unlike most spandsp types, `echo_can_init` does **not** take a
/// pointer to pre-allocated memory as its first argument. It always allocates
/// internally and returns a pointer (or NULL on failure).
pub struct EchoCanceller {
    ptr: NonNull<spandsp_sys::echo_can_state_t>,
}

impl EchoCanceller {
    /// Create a new echo canceller.
    ///
    /// - `len`: the length of the canceller in samples (tail length).
    /// - `flags`: a combination of `EchoCanFlags`.
    pub fn new(len: i32, flags: EchoCanFlags) -> Result<Self> {
        let ptr = unsafe { spandsp_sys::echo_can_init(len as c_int, flags.bits() as c_int) };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { ptr })
    }

    /// Process a single sample pair through the echo canceller.
    ///
    /// - `tx`: the transmitted (far-end) sample.
    /// - `rx`: the received (near-end) sample, which may contain echo.
    ///
    /// Returns the cleaned (echo-cancelled) receive sample.
    pub fn update(&mut self, tx: i16, rx: i16) -> i16 {
        unsafe { spandsp_sys::echo_can_update(self.ptr.as_ptr(), tx, rx) }
    }

    /// Flush (reinitialise) the echo canceller, resetting the adaptive filter.
    pub fn flush(&mut self) {
        unsafe {
            spandsp_sys::echo_can_flush(self.ptr.as_ptr());
        }
    }

    /// Change the adaption mode of the echo canceller.
    pub fn set_adaption_mode(&mut self, flags: EchoCanFlags) {
        unsafe {
            spandsp_sys::echo_can_adaption_mode(self.ptr.as_ptr(), flags.bits() as c_int);
        }
    }

    /// Apply a high-pass filter to a transmit sample.
    pub fn hpf_tx(&mut self, tx: i16) -> i16 {
        unsafe { spandsp_sys::echo_can_hpf_tx(self.ptr.as_ptr(), tx) }
    }

    /// Take a snapshot of the echo canceller state (for debugging/logging).
    pub fn snapshot(&mut self) {
        unsafe {
            spandsp_sys::echo_can_snapshot(self.ptr.as_ptr());
        }
    }

    /// Return the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::echo_can_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for EchoCanceller {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::echo_can_free(self.ptr.as_ptr());
        }
    }
}
