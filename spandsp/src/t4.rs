//! Shared types for T.4/T.6 fax image encoding and decoding.
//!
//! These types are used by both the [`t4_rx`](crate::t4_rx) (receive / decode)
//! and [`t4_tx`](crate::t4_tx) (transmit / encode) modules.

extern crate spandsp_sys;

use std::fmt;

use crate::error::SpanDspError;

// ---------------------------------------------------------------------------
// T4Compression
// ---------------------------------------------------------------------------

bitflags::bitflags! {
    /// T.4 image compression modes (bitflags).
    ///
    /// Combine with bitwise OR to indicate supported compression schemes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct T4Compression: u32 {
        /// No compression.
        const NONE = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_NONE;
        /// T.4 1D (MH) compression.
        const T4_1D = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_T4_1D;
        /// T.4 2D (MR) compression.
        const T4_2D = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_T4_2D;
        /// T.6 (MMR) compression.
        const T6 = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_T6;
        /// T.85 monochrome JBIG coding with L0 fixed.
        const T85 = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_T85;
        /// T.85 monochrome JBIG coding with L0 variable.
        const T85_L0 = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_T85_L0;
        /// T.43 grey-scale/colour JBIG coding.
        const T43 = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_T43;
        /// T.45 run-length colour coding.
        const T45 = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_T45;
        /// T.42/T.81 JPEG coding.
        const T42_T81 = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_T42_T81;
        /// T.81 sYCC JPEG coding.
        const SYCC_T81 = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_SYCC_T81;
        /// T.88 coding.
        const T88 = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_T88;
        /// Uncompressed data.
        const UNCOMPRESSED = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_UNCOMPRESSED;
        /// JPEG coding.
        const JPEG = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_JPEG;
        /// No sub-sampling modifier.
        const NO_SUBSAMPLING = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_NO_SUBSAMPLING;
        /// Grey-scale modifier.
        const GRAYSCALE = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_GRAYSCALE;
        /// Colour modifier.
        const COLOUR = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_COLOUR;
        /// 12-bit modifier.
        const BIT12 = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_12BIT;
        /// Colour-to-grey conversion modifier.
        const COLOUR_TO_GRAY = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_COLOUR_TO_GRAY;
        /// Grey-to-bilevel conversion modifier.
        const GRAY_TO_BILEVEL = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_GRAY_TO_BILEVEL;
        /// Colour-to-bilevel conversion modifier.
        const COLOUR_TO_BILEVEL = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_COLOUR_TO_BILEVEL;
        /// Rescaling modifier.
        const RESCALING = spandsp_sys::t4_image_compression_t_T4_COMPRESSION_RESCALING;
    }
}

impl fmt::Display for T4Compression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

// ---------------------------------------------------------------------------
// T4DecodeStatus
// ---------------------------------------------------------------------------

/// Status returned by T.4/T.6 decoder `put` and `put_bit` methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum T4DecodeStatus {
    /// More data is needed to complete the image.
    MoreData = spandsp_sys::t4_decoder_status_t_T4_DECODE_MORE_DATA,
    /// The image has been decoded successfully.
    Ok = spandsp_sys::t4_decoder_status_t_T4_DECODE_OK,
    /// Decoding was interrupted.
    Interrupt = spandsp_sys::t4_decoder_status_t_T4_DECODE_INTERRUPT,
    /// Decoding was aborted.
    Aborted = spandsp_sys::t4_decoder_status_t_T4_DECODE_ABORTED,
    /// Out of memory.
    NoMem = spandsp_sys::t4_decoder_status_t_T4_DECODE_NOMEM,
    /// Invalid data encountered.
    InvalidData = spandsp_sys::t4_decoder_status_t_T4_DECODE_INVALID_DATA,
}

impl From<T4DecodeStatus> for i32 {
    fn from(s: T4DecodeStatus) -> Self {
        s as i32
    }
}

impl TryFrom<i32> for T4DecodeStatus {
    type Error = SpanDspError;

    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        match value {
            x if x == Self::MoreData as i32 => Ok(Self::MoreData),
            x if x == Self::Ok as i32 => Ok(Self::Ok),
            x if x == Self::Interrupt as i32 => Ok(Self::Interrupt),
            x if x == Self::Aborted as i32 => Ok(Self::Aborted),
            x if x == Self::NoMem as i32 => Ok(Self::NoMem),
            x if x == Self::InvalidData as i32 => Ok(Self::InvalidData),
            _ => Err(SpanDspError::InvalidInput(format!(
                "invalid T4 decode status: {value}"
            ))),
        }
    }
}

impl fmt::Display for T4DecodeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::MoreData => "more-data",
            Self::Ok => "ok",
            Self::Interrupt => "interrupt",
            Self::Aborted => "aborted",
            Self::NoMem => "no-mem",
            Self::InvalidData => "invalid-data",
        };
        f.write_str(name)
    }
}

// ---------------------------------------------------------------------------
// T4Stats
// ---------------------------------------------------------------------------

/// Transfer statistics for a T.4 session.
///
/// Wraps the C `t4_stats_t` structure with idiomatic Rust field types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct T4Stats {
    /// Number of pages transferred so far.
    pub pages_transferred: i32,
    /// Number of pages in the file (negative if unknown).
    pub pages_in_file: i32,
    /// Number of bad pixel rows in the most recent page.
    pub bad_rows: i32,
    /// Largest number of bad pixel rows in a block in the most recent page.
    pub longest_bad_row_run: i32,
    /// Image type of the file page.
    pub image_type: i32,
    /// Horizontal resolution of the file page (pixels per metre).
    pub image_x_resolution: i32,
    /// Vertical resolution of the file page (pixels per metre).
    pub image_y_resolution: i32,
    /// Width of the file page (pixels).
    pub image_width: i32,
    /// Length of the file page (pixels).
    pub image_length: i32,
    /// Image type of the exchanged page.
    pub exchange_type: i32,
    /// Horizontal resolution of the exchanged page (pixels per metre).
    pub x_resolution: i32,
    /// Vertical resolution of the exchanged page (pixels per metre).
    pub y_resolution: i32,
    /// Width of the exchanged page (pixels).
    pub width: i32,
    /// Length of the exchanged page (pixels).
    pub length: i32,
    /// Compression type used between FAX machines.
    pub compression: i32,
    /// Size of the image on the line (bytes).
    pub line_image_size: i32,
}

impl From<spandsp_sys::t4_stats_t> for T4Stats {
    fn from(s: spandsp_sys::t4_stats_t) -> Self {
        Self {
            pages_transferred: s.pages_transferred,
            pages_in_file: s.pages_in_file,
            bad_rows: s.bad_rows,
            longest_bad_row_run: s.longest_bad_row_run,
            image_type: s.image_type,
            image_x_resolution: s.image_x_resolution,
            image_y_resolution: s.image_y_resolution,
            image_width: s.image_width,
            image_length: s.image_length,
            exchange_type: s.type_,
            x_resolution: s.x_resolution,
            y_resolution: s.y_resolution,
            width: s.width,
            length: s.length,
            compression: s.compression,
            line_image_size: s.line_image_size,
        }
    }
}
