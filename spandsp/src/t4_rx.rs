//! T.4 image receive (decoding) support.
//!
//! - [`T4Rx`] wraps `t4_rx_state_t` for high-level file-based receive
//!   (compressed fax data → TIFF file).
//! - [`T4T6Decoder`] wraps `t4_t6_decode_state_t` for low-level
//!   decompression (compressed bits → raw image rows via callback).

extern crate spandsp_sys;

use std::ffi::CString;
use std::os::raw::{c_int, c_void};
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};
use crate::logging::LoggingState;
use crate::t4::{T4Compression, T4DecodeStatus, T4Stats};

// ---------------------------------------------------------------------------
// Row-write callback trampoline (shared by T4Rx and T4T6Decoder)
// ---------------------------------------------------------------------------

type RowWriteCallback = Box<dyn FnMut(&[u8]) -> bool>;

/// Trampoline for `t4_row_write_handler_t`.
///
/// # Safety
///
/// `user_data` must point to a valid `RowWriteCallback`.
unsafe extern "C" fn row_write_trampoline(
    user_data: *mut c_void,
    buf: *const u8,
    len: usize,
) -> c_int {
    unsafe {
        if user_data.is_null() {
            return 0;
        }
        let closure = &mut *(user_data as *mut RowWriteCallback);
        let slice = if buf.is_null() || len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(buf, len)
        };
        if closure(slice) { 0 } else { -1 }
    }
}

// ---------------------------------------------------------------------------
// T4Rx — high-level file-based receiver
// ---------------------------------------------------------------------------

/// RAII wrapper around `t4_rx_state_t`.
///
/// Receives compressed fax data and writes decoded pages to a TIFF file.
/// Created via [`T4Rx::new()`]. Freed on drop via `t4_rx_free`.
pub struct T4Rx {
    ptr: NonNull<spandsp_sys::t4_rx_state_t>,
}

impl T4Rx {
    /// Create a new T.4 receiver that writes pages to `file`.
    ///
    /// - `file`: path to the output TIFF file.
    /// - `compressions`: supported output compression schemes.
    pub fn new(file: &str, compressions: T4Compression) -> Result<Self> {
        let c_file = CString::new(file)
            .map_err(|_| SpanDspError::InvalidInput("file path contains NUL byte".into()))?;
        let ptr = unsafe {
            spandsp_sys::t4_rx_init(
                std::ptr::null_mut(),
                c_file.as_ptr(),
                compressions.bits() as c_int,
            )
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { ptr })
    }

    /// Prepare to receive the next page.
    pub fn start_page(&mut self) -> Result<()> {
        let rc = unsafe { spandsp_sys::t4_rx_start_page(self.ptr.as_ptr()) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Feed a block of compressed data to the receiver.
    pub fn put(&mut self, buf: &[u8]) -> T4DecodeStatus {
        let rc = unsafe { spandsp_sys::t4_rx_put(self.ptr.as_ptr(), buf.as_ptr(), buf.len()) };
        T4DecodeStatus::try_from(rc).unwrap_or(T4DecodeStatus::InvalidData)
    }

    /// Feed a single bit of compressed data to the receiver.
    pub fn put_bit(&mut self, bit: i32) -> T4DecodeStatus {
        let rc = unsafe { spandsp_sys::t4_rx_put_bit(self.ptr.as_ptr(), bit as c_int) };
        T4DecodeStatus::try_from(rc).unwrap_or(T4DecodeStatus::InvalidData)
    }

    /// Complete reception of the current page.
    pub fn end_page(&mut self) -> Result<()> {
        let rc = unsafe { spandsp_sys::t4_rx_end_page(self.ptr.as_ptr()) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Set the encoding for received data.
    pub fn set_rx_encoding(&mut self, encoding: T4Compression) -> Result<()> {
        let rc = unsafe {
            spandsp_sys::t4_rx_set_rx_encoding(self.ptr.as_ptr(), encoding.bits() as c_int)
        };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Set the expected width of the received image in pixel columns.
    pub fn set_image_width(&mut self, width: i32) {
        unsafe {
            spandsp_sys::t4_rx_set_image_width(self.ptr.as_ptr(), width as c_int);
        }
    }

    /// Set the column-to-column (x) resolution in pixels per metre.
    pub fn set_x_resolution(&mut self, resolution: i32) {
        unsafe {
            spandsp_sys::t4_rx_set_x_resolution(self.ptr.as_ptr(), resolution as c_int);
        }
    }

    /// Set the row-to-row (y) resolution in pixels per metre.
    pub fn set_y_resolution(&mut self, resolution: i32) {
        unsafe {
            spandsp_sys::t4_rx_set_y_resolution(self.ptr.as_ptr(), resolution as c_int);
        }
    }

    /// Set the DCS information string, for inclusion in the file.
    pub fn set_dcs(&mut self, dcs: &str) -> Result<()> {
        let c_dcs = CString::new(dcs)
            .map_err(|_| SpanDspError::InvalidInput("DCS contains NUL byte".into()))?;
        unsafe {
            spandsp_sys::t4_rx_set_dcs(self.ptr.as_ptr(), c_dcs.as_ptr());
        }
        Ok(())
    }

    /// Set the sub-address of the fax, for inclusion in the file.
    pub fn set_sub_address(&mut self, sub_address: &str) -> Result<()> {
        let c_sub = CString::new(sub_address)
            .map_err(|_| SpanDspError::InvalidInput("sub-address contains NUL byte".into()))?;
        unsafe {
            spandsp_sys::t4_rx_set_sub_address(self.ptr.as_ptr(), c_sub.as_ptr());
        }
        Ok(())
    }

    /// Set the identity of the remote machine, for inclusion in the file.
    pub fn set_far_ident(&mut self, ident: &str) -> Result<()> {
        let c_ident = CString::new(ident)
            .map_err(|_| SpanDspError::InvalidInput("far ident contains NUL byte".into()))?;
        unsafe {
            spandsp_sys::t4_rx_set_far_ident(self.ptr.as_ptr(), c_ident.as_ptr());
        }
        Ok(())
    }

    /// Set the vendor of the remote machine, for inclusion in the file.
    pub fn set_vendor(&mut self, vendor: &str) -> Result<()> {
        let c_vendor = CString::new(vendor)
            .map_err(|_| SpanDspError::InvalidInput("vendor contains NUL byte".into()))?;
        unsafe {
            spandsp_sys::t4_rx_set_vendor(self.ptr.as_ptr(), c_vendor.as_ptr());
        }
        Ok(())
    }

    /// Set the model of the remote machine, for inclusion in the file.
    pub fn set_model(&mut self, model: &str) -> Result<()> {
        let c_model = CString::new(model)
            .map_err(|_| SpanDspError::InvalidInput("model contains NUL byte".into()))?;
        unsafe {
            spandsp_sys::t4_rx_set_model(self.ptr.as_ptr(), c_model.as_ptr());
        }
        Ok(())
    }

    /// Get the current image transfer statistics.
    pub fn get_transfer_statistics(&self) -> T4Stats {
        let mut stats = std::mem::MaybeUninit::<spandsp_sys::t4_stats_t>::zeroed();
        unsafe {
            spandsp_sys::t4_rx_get_transfer_statistics(self.ptr.as_ptr(), stats.as_mut_ptr());
            T4Stats::from(stats.assume_init())
        }
    }

    /// Get the logging state associated with this receiver.
    ///
    /// # Safety
    ///
    /// The returned [`LoggingState`] borrows from this `T4Rx` and must not
    /// outlive it. The caller must ensure it is not used after this object
    /// is dropped.
    pub unsafe fn get_logging_state(&self) -> LoggingState {
        let ptr = unsafe { spandsp_sys::t4_rx_get_logging_state(self.ptr.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("t4_rx_get_logging_state returned NULL");
        unsafe { LoggingState::from_ptr_borrowed(ptr) }
    }

    /// Return the raw pointer to the underlying state.
    pub fn as_ptr(&self) -> *mut spandsp_sys::t4_rx_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for T4Rx {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::t4_rx_free(self.ptr.as_ptr());
        }
    }
}

// ---------------------------------------------------------------------------
// T4T6Decoder — low-level decompressor
// ---------------------------------------------------------------------------

/// RAII wrapper around `t4_t6_decode_state_t`.
///
/// Decompresses T.4/T.6 encoded data, delivering decoded image rows via a
/// callback. No file I/O is involved.
///
/// Created via [`T4T6Decoder::new()`]. Freed on drop via `t4_t6_decode_free`.
pub struct T4T6Decoder {
    ptr: NonNull<spandsp_sys::t4_t6_decode_state_t>,
    _callback: Option<Box<RowWriteCallback>>,
}

impl T4T6Decoder {
    /// Create a new T.4/T.6 decoder.
    ///
    /// - `encoding`: the compression encoding to decode.
    /// - `image_width`: the image width in pixels.
    /// - `handler`: closure called for each decoded row. Receives the row
    ///   pixel data as `&[u8]`. Return `true` to continue, `false` to abort.
    pub fn new<F>(encoding: T4Compression, image_width: i32, handler: F) -> Result<Self>
    where
        F: FnMut(&[u8]) -> bool + 'static,
    {
        let boxed: Box<RowWriteCallback> = Box::new(Box::new(handler));
        let user_data = &*boxed as *const RowWriteCallback as *mut c_void;
        let ptr = unsafe {
            spandsp_sys::t4_t6_decode_init(
                std::ptr::null_mut(),
                encoding.bits() as c_int,
                image_width as c_int,
                Some(row_write_trampoline),
                user_data,
            )
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self {
            ptr,
            _callback: Some(boxed),
        })
    }

    /// Feed a block of compressed data to the decoder.
    pub fn put(&mut self, buf: &[u8]) -> T4DecodeStatus {
        let rc =
            unsafe { spandsp_sys::t4_t6_decode_put(self.ptr.as_ptr(), buf.as_ptr(), buf.len()) };
        T4DecodeStatus::try_from(rc).unwrap_or(T4DecodeStatus::InvalidData)
    }

    /// Feed a single bit of compressed data to the decoder.
    pub fn put_bit(&mut self, bit: i32) -> T4DecodeStatus {
        let rc = unsafe { spandsp_sys::t4_t6_decode_put_bit(self.ptr.as_ptr(), bit as c_int) };
        T4DecodeStatus::try_from(rc).unwrap_or(T4DecodeStatus::InvalidData)
    }

    /// Restart the decoder with a new image width.
    pub fn restart(&mut self, image_width: i32) -> Result<()> {
        let rc =
            unsafe { spandsp_sys::t4_t6_decode_restart(self.ptr.as_ptr(), image_width as c_int) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Set the encoding for the compressed data.
    pub fn set_encoding(&mut self, encoding: T4Compression) -> Result<()> {
        let rc = unsafe {
            spandsp_sys::t4_t6_decode_set_encoding(self.ptr.as_ptr(), encoding.bits() as c_int)
        };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Get the width of the image in pixels.
    pub fn image_width(&self) -> u32 {
        unsafe { spandsp_sys::t4_t6_decode_get_image_width(self.ptr.as_ptr()) }
    }

    /// Get the length of the image in pixels.
    pub fn image_length(&self) -> u32 {
        unsafe { spandsp_sys::t4_t6_decode_get_image_length(self.ptr.as_ptr()) }
    }

    /// Get the size of the compressed image in bits.
    pub fn compressed_image_size(&self) -> i32 {
        unsafe { spandsp_sys::t4_t6_decode_get_compressed_image_size(self.ptr.as_ptr()) }
    }

    /// Get the logging state associated with this decoder.
    ///
    /// # Safety
    ///
    /// The returned [`LoggingState`] borrows from this `T4T6Decoder` and must
    /// not outlive it.
    pub unsafe fn get_logging_state(&self) -> LoggingState {
        let ptr = unsafe { spandsp_sys::t4_t6_decode_get_logging_state(self.ptr.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("t4_t6_decode_get_logging_state returned NULL");
        unsafe { LoggingState::from_ptr_borrowed(ptr) }
    }

    /// Return the raw pointer to the underlying state.
    pub fn as_ptr(&self) -> *mut spandsp_sys::t4_t6_decode_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for T4T6Decoder {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::t4_t6_decode_free(self.ptr.as_ptr());
        }
    }
}
