//! Safe wrapper around the T.38 gateway.
//!
//! A T.38 gateway bridges between analog FAX (audio samples) and
//! T.38 IP packets, allowing traditional PSTN FAX machines to
//! communicate through an IP network.

use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};
use crate::t30::T30ModemSupport;
use crate::t38_core::T38Core;

/// T.38 gateway state wrapping `t38_gateway_state_t`.
pub struct T38Gateway {
    inner: NonNull<spandsp_sys::t38_gateway_state_t>,
}

impl T38Gateway {
    /// Create a new T.38 gateway.
    ///
    /// # Safety
    /// `tx_packet_handler` and `tx_packet_user_data` must remain valid for
    /// the lifetime of this object.
    pub unsafe fn new_raw(
        tx_packet_handler: spandsp_sys::t38_tx_packet_handler_t,
        tx_packet_user_data: *mut std::ffi::c_void,
    ) -> Result<Self> {
        unsafe {
            let ptr = spandsp_sys::t38_gateway_init(
                std::ptr::null_mut(),
                tx_packet_handler,
                tx_packet_user_data,
            );
            let inner = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
            Ok(Self { inner })
        }
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::t38_gateway_state_t {
        self.inner.as_ptr()
    }

    /// Get a (non-owned) handle to the T.38 core IFP engine.
    pub fn get_t38_core_state(&self) -> Result<T38Core> {
        let ptr = unsafe { spandsp_sys::t38_gateway_get_t38_core_state(self.inner.as_ptr()) };
        unsafe { T38Core::from_raw(ptr) }
    }

    /// Process received audio samples (PSTN side → T.38).
    ///
    /// Returns the number of unprocessed samples.
    pub fn rx(&self, samples: &mut [i16]) -> usize {
        unsafe {
            spandsp_sys::t38_gateway_rx(
                self.inner.as_ptr(),
                samples.as_mut_ptr(),
                samples.len() as i32,
            ) as usize
        }
    }

    /// Generate transmit audio samples (T.38 → PSTN side).
    ///
    /// Returns the number of samples generated.
    pub fn tx(&self, buf: &mut [i16]) -> usize {
        unsafe {
            spandsp_sys::t38_gateway_tx(self.inner.as_ptr(), buf.as_mut_ptr(), buf.len() as i32)
                as usize
        }
    }

    /// Set whether ECM is allowed.
    pub fn set_ecm_capability(&self, allowed: bool) {
        unsafe {
            spandsp_sys::t38_gateway_set_ecm_capability(self.inner.as_ptr(), allowed);
        }
    }

    /// Set whether to send silent audio when idle.
    pub fn set_transmit_on_idle(&self, on: bool) {
        unsafe {
            spandsp_sys::t38_gateway_set_transmit_on_idle(self.inner.as_ptr(), on);
        }
    }

    /// Set supported modems.
    pub fn set_supported_modems(&self, modems: T30ModemSupport) {
        unsafe {
            spandsp_sys::t38_gateway_set_supported_modems(self.inner.as_ptr(), modems.bits());
        }
    }

    /// Set TEP mode.
    pub fn set_tep_mode(&self, use_tep: bool) {
        unsafe {
            spandsp_sys::t38_gateway_set_tep_mode(self.inner.as_ptr(), use_tep);
        }
    }

    /// Get transfer statistics.
    pub fn get_transfer_statistics(&self) -> spandsp_sys::t38_stats_t {
        let mut stats = unsafe { std::mem::zeroed::<spandsp_sys::t38_stats_t>() };
        unsafe {
            spandsp_sys::t38_gateway_get_transfer_statistics(self.inner.as_ptr(), &mut stats);
        }
        stats
    }
}

impl Drop for T38Gateway {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::t38_gateway_free(self.inner.as_ptr());
        }
    }
}
