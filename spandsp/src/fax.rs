//! Safe wrapper around the high-level analog FAX state machine.
//!
//! `FaxState` combines the T.30 protocol engine with FAX modems for
//! analog line FAX operation.

use std::os::raw::c_int;
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};
use crate::t30::T30State;
use crate::{ReceiveRecovery, ReceiveReport};

/// High-level analog FAX state wrapping `fax_state_t`.
///
/// Created via `FaxState::new()`, freed on drop.
pub struct FaxState {
    inner: NonNull<spandsp_sys::fax_state_t>,
    report: Option<ReceiveReport>,
}

impl FaxState {
    /// Create a new FAX context.
    ///
    /// `calling_party` — true for the originating side, false for answering.
    pub fn new(calling_party: bool) -> Result<Self> {
        let ptr = unsafe { spandsp_sys::fax_init(std::ptr::null_mut(), calling_party) };
        let inner = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self {
            inner,
            report: None,
        })
    }

    /// Construct an answering receiver with its policy fixed before any media.
    pub fn new_receiver(policy: ReceiveRecovery) -> Result<Self> {
        let state = Self::new(false)?;
        unsafe {
            spandsp_sys::t30_set_receive_recovery(
                spandsp_sys::fax_get_t30_state(state.inner.as_ptr()),
                (policy == ReceiveRecovery::PreserveDecodedRows) as i32,
            );
        }
        Ok(state)
    }

    /// Stop reception and close TIFF output, without synthesizing phase E.
    /// Repeated calls return the same cached report and never append pages.
    /// The owner cannot be restarted afterward. All borrowed handles must have
    /// ended; callback user data may be released after this call returns.
    pub fn finalize_receive(&mut self) -> &ReceiveReport {
        if self.report.is_none() {
            self.report = Some(unsafe {
                crate::receive_recovery::finalize(spandsp_sys::fax_get_t30_state(
                    self.inner.as_ptr(),
                ))
            });
        }
        self.report.as_ref().unwrap()
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::fax_state_t {
        self.inner.as_ptr()
    }

    /// Borrow a handle to the T.30 protocol engine inside this FAX context.
    pub fn get_t30_state(&self) -> Result<T30State<'_>> {
        let ptr = unsafe { spandsp_sys::fax_get_t30_state(self.inner.as_ptr()) };
        unsafe { T30State::from_raw(ptr, false) }
    }

    /// Process received audio samples through the FAX engine.
    ///
    /// Returns the number of unprocessed samples (non-zero means end of call).
    pub fn rx(&self, samples: &mut [i16]) -> usize {
        if self.report.is_some() {
            return samples.len();
        }
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
        if self.report.is_some() {
            return 0;
        }
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
    pub fn restart(&mut self, calling_party: bool) -> Result<()> {
        if self.report.is_some() {
            return Err(SpanDspError::InvalidInput("receiver finalized".into()));
        }
        let rc = unsafe { spandsp_sys::fax_restart(self.inner.as_ptr(), calling_party) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }
}

// SAFETY: FaxState wraps a SpanDSP fax_state_t that is only accessed through
// &self/&mut self methods. The underlying C library is not thread-safe, but
// exclusive access can be guaranteed externally (e.g., via tokio::sync::Mutex).
unsafe impl Send for FaxState {}

impl Drop for FaxState {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::fax_free(self.inner.as_ptr());
        }
    }
}
