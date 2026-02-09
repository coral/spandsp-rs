//! Safe wrapper around the T.38 terminal endpoint.
//!
//! A T.38 terminal is an Internet-aware FAX device that connects directly
//! to an IP network, sending and receiving T.38 IFP packets.

use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};
use crate::t30::T30State;
use crate::t38_core::{T38Core, T38TerminalOptions};

/// T.38 terminal state wrapping `t38_terminal_state_t`.
pub struct T38Terminal {
    inner: NonNull<spandsp_sys::t38_terminal_state_t>,
}

impl T38Terminal {
    /// Create a new T.38 terminal.
    ///
    /// # Safety
    /// `tx_packet_handler` and `tx_packet_user_data` must remain valid for
    /// the lifetime of this object.
    pub unsafe fn new_raw(
        calling_party: bool,
        tx_packet_handler: spandsp_sys::t38_tx_packet_handler_t,
        tx_packet_user_data: *mut std::ffi::c_void,
    ) -> Result<Self> {
        unsafe {
            let ptr = spandsp_sys::t38_terminal_init(
                std::ptr::null_mut(),
                calling_party,
                tx_packet_handler,
                tx_packet_user_data,
            );
            let inner = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
            Ok(Self { inner })
        }
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::t38_terminal_state_t {
        self.inner.as_ptr()
    }

    /// Get a (non-owned) handle to the T.30 engine.
    pub fn get_t30_state(&self) -> Result<T30State> {
        let ptr = unsafe { spandsp_sys::t38_terminal_get_t30_state(self.inner.as_ptr()) };
        unsafe { T30State::from_raw(ptr, false) }
    }

    /// Get a (non-owned) handle to the T.38 core IFP engine.
    pub fn get_t38_core_state(&self) -> Result<T38Core> {
        let ptr = unsafe { spandsp_sys::t38_terminal_get_t38_core_state(self.inner.as_ptr()) };
        unsafe { T38Core::from_raw(ptr) }
    }

    /// Drive the T.38 terminal's timer. Call periodically with the number of
    /// audio-equivalent samples elapsed.
    pub fn send_timeout(&self, samples: i32) -> i32 {
        unsafe { spandsp_sys::t38_terminal_send_timeout(self.inner.as_ptr(), samples) }
    }

    /// Set configuration options.
    pub fn set_config(&self, config: T38TerminalOptions) {
        unsafe {
            spandsp_sys::t38_terminal_set_config(self.inner.as_ptr(), config.bits());
        }
    }

    /// Set whether TEP (Talker Echo Protection) time is allowed for.
    pub fn set_tep_mode(&self, use_tep: bool) {
        unsafe {
            spandsp_sys::t38_terminal_set_tep_mode(self.inner.as_ptr(), use_tep);
        }
    }

    /// Set fill bit removal mode.
    pub fn set_fill_bit_removal(&self, remove: bool) {
        unsafe {
            spandsp_sys::t38_terminal_set_fill_bit_removal(self.inner.as_ptr(), remove);
        }
    }

    /// Restart the terminal.
    pub fn restart(&self, calling_party: bool) -> Result<()> {
        let rc = unsafe { spandsp_sys::t38_terminal_restart(self.inner.as_ptr(), calling_party) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }
}

// SAFETY: T38Terminal wraps a SpanDSP t38_terminal_state_t that is only accessed
// through &self/&mut self methods. The underlying C library is not thread-safe,
// but exclusive access can be guaranteed externally (e.g., via tokio::sync::Mutex).
unsafe impl Send for T38Terminal {}

impl Drop for T38Terminal {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::t38_terminal_free(self.inner.as_ptr());
        }
    }
}
