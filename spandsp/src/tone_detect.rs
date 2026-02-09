//! Safe wrappers around spandsp's Goertzel tone detection.
//!
//! The Goertzel algorithm efficiently computes a single DFT bin, making it
//! ideal for detecting specific frequencies (e.g. DTMF tones).

extern crate spandsp_sys;

use std::os::raw::c_int;
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};

/// Descriptor for a Goertzel filter, specifying the target frequency and
/// block size.
///
/// This is a stack-allocated value type (not heap-allocated by spandsp).
pub struct GoertzelDescriptor {
    inner: spandsp_sys::goertzel_descriptor_t,
}

impl GoertzelDescriptor {
    /// Create a Goertzel descriptor for the given frequency and block size.
    ///
    /// - `freq`: the target frequency in Hz.
    /// - `samples`: the number of samples per Goertzel block.
    pub fn new(freq: f32, samples: usize) -> Self {
        let mut desc = spandsp_sys::goertzel_descriptor_t::default();
        unsafe {
            spandsp_sys::make_goertzel_descriptor(&mut desc, freq, samples as c_int);
        }
        Self { inner: desc }
    }

    /// Return a mutable pointer to the inner descriptor (for passing to FFI).
    pub fn as_mut_ptr(&mut self) -> *mut spandsp_sys::goertzel_descriptor_t {
        &mut self.inner
    }
}

/// RAII wrapper around `goertzel_state_t`.
///
/// Created via `GoertzelDetector::new()`, which calls
/// `goertzel_init(NULL, ...)`. Freed on drop via `goertzel_free`.
pub struct GoertzelDetector {
    ptr: NonNull<spandsp_sys::goertzel_state_t>,
}

impl GoertzelDetector {
    /// Create a new Goertzel detector from a descriptor.
    pub fn new(desc: &mut GoertzelDescriptor) -> Result<Self> {
        let ptr = unsafe { spandsp_sys::goertzel_init(std::ptr::null_mut(), desc.as_mut_ptr()) };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { ptr })
    }

    /// Reset the detector state so it can be reused for a new block.
    pub fn reset(&mut self) {
        unsafe {
            spandsp_sys::goertzel_reset(self.ptr.as_ptr());
        }
    }

    /// Feed audio samples to the Goertzel detector.
    ///
    /// Returns the number of unprocessed samples.
    pub fn update(&mut self, amp: &[i16]) -> usize {
        let samples = amp.len().min(c_int::MAX as usize) as c_int;
        unsafe { spandsp_sys::goertzel_update(self.ptr.as_ptr(), amp.as_ptr(), samples) as usize }
    }

    /// Evaluate the final result of the Goertzel transform for the current
    /// block.
    ///
    /// The returned value is proportional to the power at the target
    /// frequency. Call `reset()` before starting the next block.
    pub fn result(&mut self) -> f32 {
        unsafe { spandsp_sys::goertzel_result(self.ptr.as_ptr()) }
    }

    /// Return the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::goertzel_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for GoertzelDetector {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::goertzel_free(self.ptr.as_ptr());
        }
    }
}
