//! Explicit receiver shutdown and an inventory of closed TIFF output.
//!
//! Completed pages retain ordinary decoder output, including repaired rows and
//! the good rows after them. Their damage count is reported separately.
//! For unfinished pages, preservation retains a contiguous trustworthy prefix
//! for T.4 1D/2D, T.6, and T.85, stopping at the first damaged row or missing frame.
//! Interrupted-page salvage emits no concealed rows or synthetic flush bits.
//! Other codecs retain completed pages, but an unfinished page is unsupported
//! and omitted.

//! Borrowed native handles cannot outlive their owner.
//!
//! ```compile_fail
//! let fax = spandsp::fax::FaxState::new(false).unwrap();
//! let t30 = fax.get_t30_state().unwrap();
//! drop(fax);
//! t30.call_active();
//! ```
//!
//! Finalization also requires all live borrowed handles to end.
//!
//! ```compile_fail
//! let mut fax = spandsp::fax::FaxState::new(false).unwrap();
//! let t30 = fax.get_t30_state().unwrap();
//! fax.finalize_receive();
//! t30.call_active();
//! ```
//!
//! ```compile_fail
//! use spandsp::t38_terminal::T38Terminal;
//! unsafe extern "C" fn packet(_: *mut spandsp::spandsp_sys::t38_core_state_t,
//!     _: *mut std::ffi::c_void, _: *const u8, _: i32, _: i32) -> i32 { 0 }
//! let mut terminal = unsafe { T38Terminal::new_raw(false, Some(packet), std::ptr::null_mut()).unwrap() };
//! let core = terminal.get_t38_core_state().unwrap();
//! terminal.finalize_receive();
//! core.rx_ifp_packet(&[], 0);
//! ```

/// Receiver policy, selected at construction before media is processed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveRecovery {
    /// Retain the library's ordinary receive behavior.
    #[default]
    Disabled,
    /// Preserve the decodable prefix of an unfinished bilevel page.
    PreserveDecodedRows,
}

/// Protocol completion is independent of the TIFF inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveCompletion {
    /// Actual phase E result, including failure codes.
    Completed { code: i32 },
    /// Finalized before phase E; the last native status may still be zero.
    Interrupted { last_status: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceivePageKind {
    Complete,
    RecoveredPartial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivePage {
    pub kind: ReceivePageKind,
    pub decoded_rows: u32,
    pub pixel_width: u32,
    /// Pixels per metre (not DPI).
    pub horizontal_resolution: i32,
    /// Pixels per metre (not DPI).
    pub vertical_resolution: i32,
    pub decoder_bad_rows: u32,
    /// An omitted tail has unknown length. No blank rows stand in for it.
    pub missing_tail: bool,
}

bitflags::bitflags! {
    /// Independent output errors. Multiple operations can fail.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ReceiveOutputError: i32 {
        const WRITE = 1;
        const FLUSH = 2;
        const CLOSE = 4;
        const ALLOCATION = 8;
        const OPEN = 16;
    }
}

#[derive(Debug, Clone)]
pub struct ReceiveReport {
    pub completion: ReceiveCompletion,
    /// Original T.30 pages_rx; salvage never increments this count.
    pub confirmed_complete_pages: u32,
    /// TIFF directories, in output order. Only successfully written images appear.
    pub pages: Vec<ReceivePage>,
    /// True when flush and file closure succeeded, or no output was opened.
    pub output_closed: bool,
    pub output_error: Option<ReceiveOutputError>,
    pub missing_tail: bool,
    pub unsupported_partial_codec: bool,
    /// Unmodified statistics captured before explicit finalization.
    pub statistics: spandsp_sys::t30_stats_t,
}

pub(crate) unsafe fn finalize(ptr: *mut spandsp_sys::t30_state_t) -> ReceiveReport {
    unsafe {
        spandsp_sys::t30_receive_finalize(ptr);
        let mut statistics = std::mem::zeroed();
        spandsp_sys::t30_get_transfer_statistics(ptr, &mut statistics);
        let completion = if spandsp_sys::t30_receive_completed(ptr) != 0 {
            ReceiveCompletion::Completed {
                code: spandsp_sys::t30_receive_completion_code(ptr),
            }
        } else {
            ReceiveCompletion::Interrupted {
                last_status: statistics.current_status,
            }
        };
        let pages = (0..spandsp_sys::t30_receive_page_count(ptr))
            .map(|i| {
                let mut page = std::mem::zeroed::<spandsp_sys::t4_rx_recovery_page_t>();
                spandsp_sys::t30_receive_page(ptr, i, &mut page);
                ReceivePage {
                    kind: if page.partial != 0 {
                        ReceivePageKind::RecoveredPartial
                    } else {
                        ReceivePageKind::Complete
                    },
                    decoded_rows: page.rows as u32,
                    pixel_width: page.width as u32,
                    horizontal_resolution: page.x_resolution,
                    vertical_resolution: page.y_resolution,
                    decoder_bad_rows: page.bad_rows as u32,
                    missing_tail: page.missing_tail != 0,
                }
            })
            .collect();
        let errors =
            ReceiveOutputError::from_bits_retain(spandsp_sys::t30_receive_output_error(ptr));
        ReceiveReport {
            completion,
            confirmed_complete_pages: statistics.pages_rx as u32,
            pages,
            output_closed: spandsp_sys::t30_receive_output_closed(ptr) != 0,
            output_error: if errors.is_empty() {
                None
            } else {
                Some(errors)
            },
            missing_tail: spandsp_sys::t30_receive_missing_tail(ptr) != 0,
            unsupported_partial_codec: spandsp_sys::t30_receive_unsupported_partial(ptr) != 0,
            statistics,
        }
    }
}
