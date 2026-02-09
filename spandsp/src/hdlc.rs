//! Safe wrappers around spandsp's HDLC framing and deframing.
//!
//! - `HdlcTx` wraps `hdlc_tx_state_t` for HDLC transmit (bit-stuffing, CRC).
//! - `HdlcRx` wraps `hdlc_rx_state_t` for HDLC receive (destuffing, CRC check).

extern crate spandsp_sys;

use std::os::raw::{c_int, c_void};
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};

type HdlcRxCallback = Box<dyn FnMut(&[u8], bool)>;
type HdlcTxCallback = Box<dyn FnMut()>;

// ---------------------------------------------------------------------------
// HdlcRx
// ---------------------------------------------------------------------------

/// Trampoline for the HDLC frame received callback.
///
/// # Safety
///
/// `user_data` must point to a valid `HdlcRxCallback`.
unsafe extern "C" fn hdlc_rx_frame_trampoline(
    user_data: *mut c_void,
    pkt: *const u8,
    len: c_int,
    ok: c_int,
) {
    unsafe {
        if user_data.is_null() {
            return;
        }
        let closure = &mut *(user_data as *mut HdlcRxCallback);
        if pkt.is_null() || len <= 0 {
            closure(&[], ok != 0);
        } else {
            let data = std::slice::from_raw_parts(pkt, len as usize);
            closure(data, ok != 0);
        }
    }
}

/// RAII wrapper around `hdlc_rx_state_t`.
///
/// Created via `HdlcRx::new()`. Freed on drop via `hdlc_rx_free`.
pub struct HdlcRx {
    ptr: NonNull<spandsp_sys::hdlc_rx_state_t>,
    _callback: Option<Box<HdlcRxCallback>>,
}

impl HdlcRx {
    /// Create a new HDLC receiver.
    ///
    /// - `crc32`: `true` for ITU CRC-32, `false` for ITU CRC-16.
    /// - `report_bad_frames`: `true` to deliver frames that fail CRC.
    /// - `framing_ok_threshold`: number of consecutive flags required before
    ///   framing is considered OK.
    /// - `handler`: closure called for each received frame. Arguments are
    ///   `(frame_data, crc_ok)`.
    pub fn new<F>(
        crc32: bool,
        report_bad_frames: bool,
        framing_ok_threshold: i32,
        handler: F,
    ) -> Result<Self>
    where
        F: FnMut(&[u8], bool) + 'static,
    {
        let boxed: Box<HdlcRxCallback> = Box::new(Box::new(handler));
        let user_data = &*boxed as *const HdlcRxCallback as *mut c_void;
        let ptr = unsafe {
            spandsp_sys::hdlc_rx_init(
                std::ptr::null_mut(),
                crc32,
                report_bad_frames,
                framing_ok_threshold as c_int,
                Some(hdlc_rx_frame_trampoline),
                user_data,
            )
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self {
            ptr,
            _callback: Some(boxed),
        })
    }

    /// Feed a block of bytes to the HDLC receiver for deframing.
    pub fn put(&mut self, buf: &[u8]) {
        let len = buf.len().min(c_int::MAX as usize) as c_int;
        unsafe {
            spandsp_sys::hdlc_rx_put(self.ptr.as_ptr(), buf.as_ptr(), len);
        }
    }

    /// Feed a single bit to the HDLC receiver.
    pub fn put_bit(&mut self, bit: bool) {
        unsafe {
            spandsp_sys::hdlc_rx_put_bit(self.ptr.as_ptr(), bit as c_int);
        }
    }

    /// Feed a single byte to the HDLC receiver.
    pub fn put_byte(&mut self, byte: u8) {
        unsafe {
            spandsp_sys::hdlc_rx_put_byte(self.ptr.as_ptr(), byte as c_int);
        }
    }

    /// Restart the HDLC receiver (does not reset statistics).
    pub fn restart(&mut self) {
        unsafe {
            spandsp_sys::hdlc_rx_restart(self.ptr.as_ptr());
        }
    }

    /// Set the maximum acceptable frame length.
    pub fn set_max_frame_len(&mut self, max_len: usize) {
        unsafe {
            spandsp_sys::hdlc_rx_set_max_frame_len(self.ptr.as_ptr(), max_len);
        }
    }

    /// Return the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::hdlc_rx_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for HdlcRx {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::hdlc_rx_free(self.ptr.as_ptr());
        }
    }
}

// ---------------------------------------------------------------------------
// HdlcTx
// ---------------------------------------------------------------------------

/// Trampoline for the HDLC transmitter underflow callback.
///
/// # Safety
///
/// `user_data` must point to a valid `HdlcTxCallback`.
unsafe extern "C" fn hdlc_tx_underflow_trampoline(user_data: *mut c_void) {
    unsafe {
        if user_data.is_null() {
            return;
        }
        let closure = &mut *(user_data as *mut HdlcTxCallback);
        closure();
    }
}

/// RAII wrapper around `hdlc_tx_state_t`.
///
/// Created via `HdlcTx::new()`. Freed on drop via `hdlc_tx_free`.
pub struct HdlcTx {
    ptr: NonNull<spandsp_sys::hdlc_tx_state_t>,
    _callback: Option<Box<HdlcTxCallback>>,
}

impl HdlcTx {
    /// Create a new HDLC transmitter.
    ///
    /// - `crc32`: `true` for ITU CRC-32, `false` for ITU CRC-16.
    /// - `inter_frame_flags`: minimum flag octets between frames (typically 1).
    /// - `progressive`: `true` to allow progressive frame construction.
    /// - `underflow_handler`: optional closure called when the transmitter needs more data.
    pub fn new<F>(
        crc32: bool,
        inter_frame_flags: i32,
        progressive: bool,
        underflow_handler: Option<F>,
    ) -> Result<Self>
    where
        F: FnMut() + 'static,
    {
        let (cb, boxed): (
            spandsp_sys::hdlc_underflow_handler_t,
            Option<Box<HdlcTxCallback>>,
        ) = match underflow_handler {
            Some(h) => {
                let b: Box<HdlcTxCallback> = Box::new(Box::new(h));
                let _ud = &*b as *const HdlcTxCallback as *mut c_void;
                // We need to smuggle user_data through; spandsp stores it.
                (Some(hdlc_tx_underflow_trampoline as _), Some(b))
            }
            None => (None, None),
        };

        let user_data = match &boxed {
            Some(b) => &**b as *const HdlcTxCallback as *mut c_void,
            None => std::ptr::null_mut(),
        };

        let ptr = unsafe {
            spandsp_sys::hdlc_tx_init(
                std::ptr::null_mut(),
                crc32,
                inter_frame_flags as c_int,
                progressive,
                cb,
                user_data,
            )
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self {
            ptr,
            _callback: boxed,
        })
    }

    /// Queue a frame for transmission.
    pub fn frame(&mut self, data: &[u8]) -> Result<()> {
        let rc =
            unsafe { spandsp_sys::hdlc_tx_frame(self.ptr.as_ptr(), data.as_ptr(), data.len()) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Queue flag octets (preamble).
    ///
    /// If `len` is 0, requests that transmission terminate when buffers drain.
    pub fn flags(&mut self, len: i32) -> Result<()> {
        let rc = unsafe { spandsp_sys::hdlc_tx_flags(self.ptr.as_ptr(), len as c_int) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Send an abort sequence.
    pub fn abort(&mut self) -> Result<()> {
        let rc = unsafe { spandsp_sys::hdlc_tx_abort(self.ptr.as_ptr()) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Get the next block of bytes for transmission.
    ///
    /// Returns the number of bytes actually written to `buf`.
    pub fn get(&mut self, buf: &mut [u8]) -> usize {
        unsafe { spandsp_sys::hdlc_tx_get(self.ptr.as_ptr(), buf.as_mut_ptr(), buf.len()) as usize }
    }

    /// Get the next bit for transmission.
    pub fn get_bit(&mut self) -> i32 {
        unsafe { spandsp_sys::hdlc_tx_get_bit(self.ptr.as_ptr()) as i32 }
    }

    /// Restart the HDLC transmitter.
    pub fn restart(&mut self) {
        unsafe {
            spandsp_sys::hdlc_tx_restart(self.ptr.as_ptr());
        }
    }

    /// Return the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::hdlc_tx_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for HdlcTx {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::hdlc_tx_free(self.ptr.as_ptr());
        }
    }
}
