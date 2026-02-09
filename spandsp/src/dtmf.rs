//! Safe wrappers around spandsp's DTMF tone generation and detection.
//!
//! - `DtmfTx` wraps `dtmf_tx_state_t` for generating DTMF tones.
//! - `DtmfRx` wraps `dtmf_rx_state_t` for detecting DTMF digits.

extern crate spandsp_sys;

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};

// ---------------------------------------------------------------------------
// DtmfTx
// ---------------------------------------------------------------------------

/// Trampoline for the digits-needed (underflow) callback on the TX side.
///
/// # Safety
///
/// `user_data` must point to a valid `Box<dyn FnMut()>`.
unsafe extern "C" fn dtmf_tx_callback_trampoline(user_data: *mut c_void) {
    unsafe {
        if user_data.is_null() {
            return;
        }
        let closure = &mut *(user_data as *mut Box<dyn FnMut()>);
        closure();
    }
}

/// RAII wrapper around `dtmf_tx_state_t`.
///
/// Created via `DtmfTx::new()`, freed on drop via `dtmf_tx_free`.
pub struct DtmfTx {
    ptr: NonNull<spandsp_sys::dtmf_tx_state_t>,
    _callback: Option<Box<Box<dyn FnMut()>>>,
}

impl DtmfTx {
    /// Create a new DTMF transmitter with no underflow callback.
    pub fn new() -> Result<Self> {
        let ptr =
            unsafe { spandsp_sys::dtmf_tx_init(std::ptr::null_mut(), None, std::ptr::null_mut()) };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self {
            ptr,
            _callback: None,
        })
    }

    /// Create a new DTMF transmitter with an underflow callback that is
    /// invoked when the digit buffer empties and more digits are needed.
    pub fn with_callback<F>(callback: F) -> Result<Self>
    where
        F: FnMut() + 'static,
    {
        let boxed: Box<Box<dyn FnMut()>> = Box::new(Box::new(callback));
        let user_data = &*boxed as *const Box<dyn FnMut()> as *mut c_void;
        let ptr = unsafe {
            spandsp_sys::dtmf_tx_init(
                std::ptr::null_mut(),
                Some(dtmf_tx_callback_trampoline),
                user_data,
            )
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self {
            ptr,
            _callback: Some(boxed),
        })
    }

    /// Queue a string of DTMF digits for transmission.
    ///
    /// Valid digits: `0`-`9`, `A`-`D`, `*`, `#`.
    /// Returns the number of digits actually queued (may be fewer if the
    /// internal buffer is full).
    pub fn put(&mut self, digits: &str) -> Result<usize> {
        let c_digits = CString::new(digits)
            .map_err(|_| SpanDspError::InvalidInput("digits contain NUL byte".into()))?;
        let n =
            unsafe { spandsp_sys::dtmf_tx_put(self.ptr.as_ptr(), c_digits.as_ptr(), -1 as c_int) };
        Ok(n as usize)
    }

    /// Generate DTMF audio samples into the provided buffer.
    ///
    /// Returns the number of samples actually generated (may be fewer than
    /// `amp.len()` if the digit queue is exhausted).
    pub fn generate(&mut self, amp: &mut [i16]) -> usize {
        let max_samples = amp.len().min(c_int::MAX as usize) as c_int;
        unsafe { spandsp_sys::dtmf_tx(self.ptr.as_ptr(), amp.as_mut_ptr(), max_samples) as usize }
    }

    /// Set the transmit level and twist.
    ///
    /// `level` is the level of the low tone in dBm0.
    /// `twist` is the twist in dB.
    pub fn set_level(&mut self, level: i32, twist: i32) {
        unsafe {
            spandsp_sys::dtmf_tx_set_level(self.ptr.as_ptr(), level as c_int, twist as c_int);
        }
    }

    /// Set the on and off times for generated DTMF tones.
    ///
    /// Times are in milliseconds.
    pub fn set_timing(&mut self, on_time: i32, off_time: i32) {
        unsafe {
            spandsp_sys::dtmf_tx_set_timing(self.ptr.as_ptr(), on_time as c_int, off_time as c_int);
        }
    }

    /// Return the raw pointer to the underlying state.
    pub fn as_ptr(&self) -> *mut spandsp_sys::dtmf_tx_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for DtmfTx {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::dtmf_tx_free(self.ptr.as_ptr());
        }
    }
}

// ---------------------------------------------------------------------------
// DtmfRx
// ---------------------------------------------------------------------------

type DtmfCallback = Box<dyn FnMut(&str)>;

/// Trampoline for the digit-received callback on the RX side.
///
/// # Safety
///
/// `user_data` must point to a valid `DtmfCallback`.
unsafe extern "C" fn dtmf_rx_callback_trampoline(
    user_data: *mut c_void,
    digits: *const c_char,
    len: c_int,
) {
    unsafe {
        if user_data.is_null() || digits.is_null() || len <= 0 {
            return;
        }
        let closure = &mut *(user_data as *mut DtmfCallback);
        let slice = std::slice::from_raw_parts(digits as *const u8, len as usize);
        if let Ok(s) = std::str::from_utf8(slice) {
            closure(s);
        }
    }
}

/// RAII wrapper around `dtmf_rx_state_t`.
///
/// Created via `DtmfRx::new()`, freed on drop via `dtmf_rx_free`.
pub struct DtmfRx {
    ptr: NonNull<spandsp_sys::dtmf_rx_state_t>,
    _callback: Option<Box<DtmfCallback>>,
}

impl DtmfRx {
    /// Create a new DTMF receiver with no digit callback.
    ///
    /// Detected digits can be retrieved with `get()`.
    pub fn new() -> Result<Self> {
        let ptr =
            unsafe { spandsp_sys::dtmf_rx_init(std::ptr::null_mut(), None, std::ptr::null_mut()) };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self {
            ptr,
            _callback: None,
        })
    }

    /// Create a new DTMF receiver with a callback invoked each time one or
    /// more digits are detected.
    pub fn with_callback<F>(callback: F) -> Result<Self>
    where
        F: FnMut(&str) + 'static,
    {
        let boxed: Box<DtmfCallback> = Box::new(Box::new(callback));
        let user_data = &*boxed as *const DtmfCallback as *mut c_void;
        let ptr = unsafe {
            spandsp_sys::dtmf_rx_init(
                std::ptr::null_mut(),
                Some(dtmf_rx_callback_trampoline),
                user_data,
            )
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self {
            ptr,
            _callback: Some(boxed),
        })
    }

    /// Feed audio samples to the DTMF detector.
    ///
    /// Returns the number of unprocessed samples (normally 0).
    pub fn rx(&mut self, amp: &[i16]) -> usize {
        let samples = amp.len().min(c_int::MAX as usize) as c_int;
        unsafe { spandsp_sys::dtmf_rx(self.ptr.as_ptr(), amp.as_ptr(), samples) as usize }
    }

    /// Retrieve detected digits from the internal buffer.
    ///
    /// Returns the digits as a `String`. The internal buffer is drained by
    /// this call.
    pub fn get(&mut self, max_digits: usize) -> String {
        let max = max_digits.min(128); // MAX_DTMF_DIGITS
        let mut buf = vec![0u8; max + 1];
        let n = unsafe {
            spandsp_sys::dtmf_rx_get(
                self.ptr.as_ptr(),
                buf.as_mut_ptr() as *mut c_char,
                max as c_int,
            )
        };
        buf.truncate(n as usize);
        String::from_utf8_lossy(&buf).into_owned()
    }

    /// Get the current detection status of the last audio chunk.
    ///
    /// Returns `Some(digit)` if a digit is being detected, or `None` if
    /// no detection is active. The special value `'x'` indicates a "maybe"
    /// condition.
    pub fn status(&self) -> Option<char> {
        let raw = unsafe { spandsp_sys::dtmf_rx_status(self.ptr.as_ptr()) };
        if raw == 0 {
            None
        } else {
            Some(raw as u8 as char)
        }
    }

    /// Adjust detector parameters.
    ///
    /// - `filter_dialtone`: positive to enable dial tone filtering, 0 to
    ///   disable, negative to leave unchanged.
    /// - `twist`: acceptable twist in dB (< 0.0 to leave unchanged).
    /// - `reverse_twist`: acceptable reverse twist in dB (< 0.0 to leave unchanged).
    /// - `threshold`: minimum tone level in dBm0 (<= -99.0 to leave unchanged).
    pub fn set_parms(
        &mut self,
        filter_dialtone: i32,
        twist: f32,
        reverse_twist: f32,
        threshold: f32,
    ) {
        unsafe {
            spandsp_sys::dtmf_rx_parms(
                self.ptr.as_ptr(),
                filter_dialtone as c_int,
                twist,
                reverse_twist,
                threshold,
            );
        }
    }

    /// Return the raw pointer to the underlying state.
    pub fn as_ptr(&self) -> *mut spandsp_sys::dtmf_rx_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for DtmfRx {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::dtmf_rx_free(self.ptr.as_ptr());
        }
    }
}
