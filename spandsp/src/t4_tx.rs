//! T.4 image transmit (encoding) support.
//!
//! - [`T4Tx`] wraps `t4_tx_state_t` for high-level file-based transmit
//!   (TIFF file → compressed fax data).
//! - [`T4T6Encoder`] wraps `t4_t6_encode_state_t` for low-level
//!   compression (raw image rows via callback → compressed bits).

extern crate spandsp_sys;

use std::ffi::CString;
use std::os::raw::{c_int, c_void};
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};
use crate::logging::LoggingState;
use crate::t4::{T4Compression, T4Stats};

// ---------------------------------------------------------------------------
// Row-read callback trampoline (shared by T4Tx and T4T6Encoder)
// ---------------------------------------------------------------------------

type RowReadCallback = Box<dyn FnMut(&mut [u8]) -> usize>;

/// Trampoline for `t4_row_read_handler_t`.
///
/// # Safety
///
/// `user_data` must point to a valid `RowReadCallback`.
unsafe extern "C" fn row_read_trampoline(
    user_data: *mut c_void,
    buf: *mut u8,
    len: usize,
) -> c_int {
    unsafe {
        if user_data.is_null() {
            return 0;
        }
        let closure = &mut *(user_data as *mut RowReadCallback);
        let slice = if buf.is_null() || len == 0 {
            &mut []
        } else {
            std::slice::from_raw_parts_mut(buf, len)
        };
        let n = closure(slice);
        n as c_int
    }
}

// ---------------------------------------------------------------------------
// T4Tx — high-level file-based transmitter
// ---------------------------------------------------------------------------

/// RAII wrapper around `t4_tx_state_t`.
///
/// Reads pages from a TIFF file and produces compressed fax data.
/// Created via [`T4Tx::new()`]. Freed on drop via `t4_tx_free`.
pub struct T4Tx {
    ptr: NonNull<spandsp_sys::t4_tx_state_t>,
}

impl T4Tx {
    /// Create a new T.4 transmitter that reads pages from `file`.
    ///
    /// - `file`: path to the input TIFF file.
    /// - `start_page`: first page to send (`-1` for no restriction).
    /// - `stop_page`: last page to send (`-1` for no restriction).
    pub fn new(file: &str, start_page: i32, stop_page: i32) -> Result<Self> {
        let c_file = CString::new(file)
            .map_err(|_| SpanDspError::InvalidInput("file path contains NUL byte".into()))?;
        let ptr = unsafe {
            spandsp_sys::t4_tx_init(
                std::ptr::null_mut(),
                c_file.as_ptr(),
                start_page as c_int,
                stop_page as c_int,
            )
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { ptr })
    }

    /// Prepare to send the next page.
    pub fn start_page(&mut self) -> Result<()> {
        let rc = unsafe { spandsp_sys::t4_tx_start_page(self.ptr.as_ptr()) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Prepare the current page for a resend.
    pub fn restart_page(&mut self) -> Result<()> {
        let rc = unsafe { spandsp_sys::t4_tx_restart_page(self.ptr.as_ptr()) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Complete the sending of the current page.
    pub fn end_page(&mut self) -> Result<()> {
        let rc = unsafe { spandsp_sys::t4_tx_end_page(self.ptr.as_ptr()) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Get the next chunk of compressed data.
    ///
    /// Returns the number of bytes written to `buf`. If this is less than
    /// `buf.len()`, the end of the document has been reached.
    pub fn get(&mut self, buf: &mut [u8]) -> usize {
        let rc = unsafe { spandsp_sys::t4_tx_get(self.ptr.as_ptr(), buf.as_mut_ptr(), buf.len()) };
        rc.max(0) as usize
    }

    /// Get the next bit of compressed data.
    ///
    /// Returns 0 or 1 for data bits, or `SIG_STATUS_END_OF_DATA` when
    /// the document is complete.
    pub fn get_bit(&mut self) -> i32 {
        unsafe { spandsp_sys::t4_tx_get_bit(self.ptr.as_ptr()) }
    }

    /// Check whether the current image is complete.
    pub fn image_complete(&self) -> bool {
        unsafe { spandsp_sys::t4_tx_image_complete(self.ptr.as_ptr()) != 0 }
    }

    /// Check whether the next page has a different format.
    ///
    /// Returns `None` if there is no next page (or file error),
    /// `Some(false)` if the next page has the same format,
    /// `Some(true)` if the next page has a different format.
    pub fn next_page_has_different_format(&self) -> Option<bool> {
        let rc = unsafe { spandsp_sys::t4_tx_next_page_has_different_format(self.ptr.as_ptr()) };
        match rc {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }

    /// Get the compression for the encoded data.
    pub fn get_tx_compression(&self) -> i32 {
        unsafe { spandsp_sys::t4_tx_get_tx_compression(self.ptr.as_ptr()) }
    }

    /// Get the image type of the encoded data.
    pub fn get_tx_image_type(&self) -> i32 {
        unsafe { spandsp_sys::t4_tx_get_tx_image_type(self.ptr.as_ptr()) }
    }

    /// Get the X and Y resolution code of the current page.
    pub fn get_tx_resolution(&self) -> i32 {
        unsafe { spandsp_sys::t4_tx_get_tx_resolution(self.ptr.as_ptr()) }
    }

    /// Get the column-to-column (x) resolution of the current page.
    pub fn get_tx_x_resolution(&self) -> i32 {
        unsafe { spandsp_sys::t4_tx_get_tx_x_resolution(self.ptr.as_ptr()) }
    }

    /// Get the row-to-row (y) resolution of the current page.
    pub fn get_tx_y_resolution(&self) -> i32 {
        unsafe { spandsp_sys::t4_tx_get_tx_y_resolution(self.ptr.as_ptr()) }
    }

    /// Get the width of the encoded image in pixels.
    pub fn get_tx_image_width(&self) -> i32 {
        unsafe { spandsp_sys::t4_tx_get_tx_image_width(self.ptr.as_ptr()) }
    }

    /// Auto-select the transmission image format.
    ///
    /// - `supported_compressions`: bitfield of supported compression schemes.
    /// - `supported_image_sizes`: bitfield of supported image sizes.
    /// - `supported_bilevel_resolutions`: bitfield of supported bi-level resolutions.
    /// - `supported_colour_resolutions`: bitfield of supported colour/grey resolutions.
    pub fn set_tx_image_format(
        &mut self,
        supported_compressions: T4Compression,
        supported_image_sizes: i32,
        supported_bilevel_resolutions: i32,
        supported_colour_resolutions: i32,
    ) -> Result<()> {
        let rc = unsafe {
            spandsp_sys::t4_tx_set_tx_image_format(
                self.ptr.as_ptr(),
                supported_compressions.bits() as c_int,
                supported_image_sizes as c_int,
                supported_bilevel_resolutions as c_int,
                supported_colour_resolutions as c_int,
            )
        };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Set the minimum number of encoded bits per row.
    pub fn set_min_bits_per_row(&mut self, bits: i32) {
        unsafe {
            spandsp_sys::t4_tx_set_min_bits_per_row(self.ptr.as_ptr(), bits as c_int);
        }
    }

    /// Set the maximum number of 2D encoded rows between 1D encoded rows.
    pub fn set_max_2d_rows_per_1d_row(&mut self, max: i32) {
        unsafe {
            spandsp_sys::t4_tx_set_max_2d_rows_per_1d_row(self.ptr.as_ptr(), max as c_int);
        }
    }

    /// Set the identity of the local machine, for inclusion in page headers.
    pub fn set_local_ident(&mut self, ident: &str) -> Result<()> {
        let c_ident = CString::new(ident)
            .map_err(|_| SpanDspError::InvalidInput("local ident contains NUL byte".into()))?;
        unsafe {
            spandsp_sys::t4_tx_set_local_ident(self.ptr.as_ptr(), c_ident.as_ptr());
        }
        Ok(())
    }

    /// Set the info field included in page header lines.
    pub fn set_header_info(&mut self, info: &str) -> Result<()> {
        let c_info = CString::new(info)
            .map_err(|_| SpanDspError::InvalidInput("header info contains NUL byte".into()))?;
        unsafe {
            spandsp_sys::t4_tx_set_header_info(self.ptr.as_ptr(), c_info.as_ptr());
        }
        Ok(())
    }

    /// Set whether the page header overlays or extends the image.
    pub fn set_header_overlays_image(&mut self, overlay: bool) {
        unsafe {
            spandsp_sys::t4_tx_set_header_overlays_image(self.ptr.as_ptr(), overlay);
        }
    }

    /// Get the number of pages in the file.
    pub fn pages_in_file(&self) -> i32 {
        unsafe { spandsp_sys::t4_tx_get_pages_in_file(self.ptr.as_ptr()) }
    }

    /// Get the current page number in the file.
    pub fn current_page_in_file(&self) -> i32 {
        unsafe { spandsp_sys::t4_tx_get_current_page_in_file(self.ptr.as_ptr()) }
    }

    /// Get the current image transfer statistics.
    pub fn get_transfer_statistics(&self) -> T4Stats {
        let mut stats = std::mem::MaybeUninit::<spandsp_sys::t4_stats_t>::zeroed();
        unsafe {
            spandsp_sys::t4_tx_get_transfer_statistics(self.ptr.as_ptr(), stats.as_mut_ptr());
            T4Stats::from(stats.assume_init())
        }
    }

    /// Get the logging state associated with this transmitter.
    ///
    /// # Safety
    ///
    /// The returned [`LoggingState`] borrows from this `T4Tx` and must not
    /// outlive it. The caller must ensure it is not used after this object
    /// is dropped.
    pub unsafe fn get_logging_state(&self) -> LoggingState {
        let ptr = unsafe { spandsp_sys::t4_tx_get_logging_state(self.ptr.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("t4_tx_get_logging_state returned NULL");
        unsafe { LoggingState::from_ptr_borrowed(ptr) }
    }

    /// Return the raw pointer to the underlying state.
    pub fn as_ptr(&self) -> *mut spandsp_sys::t4_tx_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for T4Tx {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::t4_tx_free(self.ptr.as_ptr());
        }
    }
}

// ---------------------------------------------------------------------------
// T4T6Encoder — low-level compressor
// ---------------------------------------------------------------------------

/// RAII wrapper around `t4_t6_encode_state_t`.
///
/// Compresses raw image rows (supplied via callback) into T.4/T.6 encoded
/// data. No file I/O is involved.
///
/// Created via [`T4T6Encoder::new()`]. Freed on drop via `t4_t6_encode_free`.
pub struct T4T6Encoder {
    ptr: NonNull<spandsp_sys::t4_t6_encode_state_t>,
    _callback: Option<Box<RowReadCallback>>,
}

impl T4T6Encoder {
    /// Create a new T.4/T.6 encoder.
    ///
    /// - `encoding`: the compression encoding to use.
    /// - `image_width`: the image width in pixels.
    /// - `image_length`: the image length in pixels (`-1` if unknown).
    /// - `handler`: closure called to read each image row. Receives a mutable
    ///   buffer `&mut [u8]` to fill with row data. Return the number of bytes
    ///   filled, or `0` to signal end of image.
    pub fn new<F>(
        encoding: T4Compression,
        image_width: i32,
        image_length: i32,
        handler: F,
    ) -> Result<Self>
    where
        F: FnMut(&mut [u8]) -> usize + 'static,
    {
        let boxed: Box<RowReadCallback> = Box::new(Box::new(handler));
        let user_data = &*boxed as *const RowReadCallback as *mut c_void;
        let ptr = unsafe {
            spandsp_sys::t4_t6_encode_init(
                std::ptr::null_mut(),
                encoding.bits() as c_int,
                image_width as c_int,
                image_length as c_int,
                Some(row_read_trampoline),
                user_data,
            )
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self {
            ptr,
            _callback: Some(boxed),
        })
    }

    /// Get the next chunk of compressed data.
    ///
    /// Returns the number of bytes written to `buf`. If this is less than
    /// `buf.len()`, the end of the image has been reached.
    pub fn get(&mut self, buf: &mut [u8]) -> usize {
        let max_len = buf.len().min(c_int::MAX as usize) as c_int;
        let rc =
            unsafe { spandsp_sys::t4_t6_encode_get(self.ptr.as_ptr(), buf.as_mut_ptr(), max_len) };
        rc.max(0) as usize
    }

    /// Get the next bit of compressed data.
    ///
    /// Returns 0 or 1 for data bits, or `SIG_STATUS_END_OF_DATA` when
    /// the image is complete.
    pub fn get_bit(&mut self) -> i32 {
        unsafe { spandsp_sys::t4_t6_encode_get_bit(self.ptr.as_ptr()) }
    }

    /// Check whether the current image is complete.
    pub fn image_complete(&self) -> bool {
        unsafe { spandsp_sys::t4_t6_encode_image_complete(self.ptr.as_ptr()) != 0 }
    }

    /// Restart the encoder with a new image width and length.
    pub fn restart(&mut self, image_width: i32, image_length: i32) -> Result<()> {
        let rc = unsafe {
            spandsp_sys::t4_t6_encode_restart(
                self.ptr.as_ptr(),
                image_width as c_int,
                image_length as c_int,
            )
        };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Set the encoding for the compressed data.
    pub fn set_encoding(&mut self, encoding: T4Compression) -> Result<()> {
        let rc = unsafe {
            spandsp_sys::t4_t6_encode_set_encoding(self.ptr.as_ptr(), encoding.bits() as c_int)
        };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Set the image width in pixels.
    pub fn set_image_width(&mut self, width: i32) -> Result<()> {
        let rc =
            unsafe { spandsp_sys::t4_t6_encode_set_image_width(self.ptr.as_ptr(), width as c_int) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Set the image length in pixels.
    pub fn set_image_length(&mut self, length: i32) -> Result<()> {
        let rc = unsafe {
            spandsp_sys::t4_t6_encode_set_image_length(self.ptr.as_ptr(), length as c_int)
        };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Get the width of the image in pixels.
    pub fn image_width(&self) -> u32 {
        unsafe { spandsp_sys::t4_t6_encode_get_image_width(self.ptr.as_ptr()) }
    }

    /// Get the length of the image in pixels.
    pub fn image_length(&self) -> u32 {
        unsafe { spandsp_sys::t4_t6_encode_get_image_length(self.ptr.as_ptr()) }
    }

    /// Get the size of the compressed image in bits.
    pub fn compressed_image_size(&self) -> i32 {
        unsafe { spandsp_sys::t4_t6_encode_get_compressed_image_size(self.ptr.as_ptr()) }
    }

    /// Set the minimum number of encoded bits per row.
    pub fn set_min_bits_per_row(&mut self, bits: i32) {
        unsafe {
            spandsp_sys::t4_t6_encode_set_min_bits_per_row(self.ptr.as_ptr(), bits as c_int);
        }
    }

    /// Set the maximum number of 2D encoded rows between 1D encoded rows.
    pub fn set_max_2d_rows_per_1d_row(&mut self, max: i32) {
        unsafe {
            spandsp_sys::t4_t6_encode_set_max_2d_rows_per_1d_row(self.ptr.as_ptr(), max as c_int);
        }
    }

    /// Get the logging state associated with this encoder.
    ///
    /// # Safety
    ///
    /// The returned [`LoggingState`] borrows from this `T4T6Encoder` and must
    /// not outlive it.
    pub unsafe fn get_logging_state(&self) -> LoggingState {
        let ptr = unsafe { spandsp_sys::t4_t6_encode_get_logging_state(self.ptr.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("t4_t6_encode_get_logging_state returned NULL");
        unsafe { LoggingState::from_ptr_borrowed(ptr) }
    }

    /// Return the raw pointer to the underlying state.
    pub fn as_ptr(&self) -> *mut spandsp_sys::t4_t6_encode_state_t {
        self.ptr.as_ptr()
    }
}

impl Drop for T4T6Encoder {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::t4_t6_encode_free(self.ptr.as_ptr());
        }
    }
}
