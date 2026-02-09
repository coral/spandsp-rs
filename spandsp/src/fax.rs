//! Safe wrapper around the high-level analog FAX state machine.
//!
//! `FaxState` combines the T.30 protocol engine with FAX modems for
//! analog line FAX operation.

use std::os::raw::c_int;
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};
use crate::t30::T30State;

/// High-level analog FAX state wrapping `fax_state_t`.
///
/// Created via `FaxState::new()`, freed on drop.
pub struct FaxState {
    inner: NonNull<spandsp_sys::fax_state_t>,
}

impl FaxState {
    /// Create a new FAX context.
    ///
    /// `calling_party` — true for the originating side, false for answering.
    pub fn new(calling_party: bool) -> Result<Self> {
        let ptr = unsafe { spandsp_sys::fax_init(std::ptr::null_mut(), calling_party) };
        let inner = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { inner })
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::fax_state_t {
        self.inner.as_ptr()
    }

    /// Get a (non-owned) handle to the T.30 protocol engine inside this FAX context.
    pub fn get_t30_state(&self) -> Result<T30State> {
        let ptr = unsafe { spandsp_sys::fax_get_t30_state(self.inner.as_ptr()) };
        unsafe { T30State::from_raw(ptr, false) }
    }

    /// Process received audio samples through the FAX engine.
    ///
    /// Returns the number of unprocessed samples (non-zero means end of call).
    pub fn rx(&self, samples: &mut [i16]) -> usize {
        unsafe {
            spandsp_sys::fax_rx(
                self.inner.as_ptr(),
                samples.as_mut_ptr(),
                samples.len() as c_int,
            ) as usize
        }
    }

    /// Generate transmit audio samples.
    ///
    /// Returns the number of samples generated (0 when nothing to send).
    pub fn tx(&self, buf: &mut [i16]) -> usize {
        unsafe {
            spandsp_sys::fax_tx(self.inner.as_ptr(), buf.as_mut_ptr(), buf.len() as c_int) as usize
        }
    }

    /// Select whether silent audio is sent when FAX transmit is idle.
    pub fn set_transmit_on_idle(&self, on: bool) {
        unsafe {
            spandsp_sys::fax_set_transmit_on_idle(self.inner.as_ptr(), on as c_int);
        }
    }

    /// Restart the FAX context.
    pub fn restart(&self, calling_party: bool) -> Result<()> {
        let rc = unsafe { spandsp_sys::fax_restart(self.inner.as_ptr(), calling_party) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }
}

impl Drop for FaxState {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::fax_free(self.inner.as_ptr());
        }
    }
}
