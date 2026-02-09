//! Safe wrapper around the T.38 FoIP core protocol engine.
//!
//! T.38 is the ITU standard for real-time FAX over IP. The core module
//! handles IFP packet encoding/decoding and sequence number management.

use std::fmt;
use std::os::raw::c_int;
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};

/// T.38 indicator type, wrapping `t30_indicator_types_e`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct T38Indicator(pub spandsp_sys::t30_indicator_types_e);

impl T38Indicator {
    /// No signal present.
    pub const NO_SIGNAL: Self = Self(spandsp_sys::t30_indicator_types_e::T38_IND_NO_SIGNAL);
    /// CNG (calling) tone detected.
    pub const CNG: Self = Self(spandsp_sys::t30_indicator_types_e::T38_IND_CNG);
    /// CED (called) tone detected.
    pub const CED: Self = Self(spandsp_sys::t30_indicator_types_e::T38_IND_CED);
    /// V.21 preamble flags detected.
    pub const V21_PREAMBLE: Self = Self(spandsp_sys::t30_indicator_types_e::T38_IND_V21_PREAMBLE);
}

impl fmt::Display for T38Indicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use spandsp_sys::t30_indicator_types_e::*;
        let name = match self.0 {
            T38_IND_NO_SIGNAL => "no-signal",
            T38_IND_CNG => "CNG",
            T38_IND_CED => "CED",
            T38_IND_V21_PREAMBLE => "V.21-preamble",
            T38_IND_V27TER_2400_TRAINING => "V.27ter-2400-training",
            T38_IND_V27TER_4800_TRAINING => "V.27ter-4800-training",
            T38_IND_V29_7200_TRAINING => "V.29-7200-training",
            T38_IND_V29_9600_TRAINING => "V.29-9600-training",
            T38_IND_V17_7200_SHORT_TRAINING => "V.17-7200-short",
            T38_IND_V17_7200_LONG_TRAINING => "V.17-7200-long",
            T38_IND_V17_9600_SHORT_TRAINING => "V.17-9600-short",
            T38_IND_V17_9600_LONG_TRAINING => "V.17-9600-long",
            T38_IND_V17_12000_SHORT_TRAINING => "V.17-12000-short",
            T38_IND_V17_12000_LONG_TRAINING => "V.17-12000-long",
            T38_IND_V17_14400_SHORT_TRAINING => "V.17-14400-short",
            T38_IND_V17_14400_LONG_TRAINING => "V.17-14400-long",
            T38_IND_V8_ANSAM => "V.8-ansam",
            T38_IND_V8_SIGNAL => "V.8-signal",
            T38_IND_V34_CNTL_CHANNEL_1200 => "V.34-cc-1200",
            T38_IND_V34_PRI_CHANNEL => "V.34-pri-channel",
            T38_IND_V34_CC_RETRAIN => "V.34-cc-retrain",
            T38_IND_V33_12000_TRAINING => "V.33-12000-training",
            T38_IND_V33_14400_TRAINING => "V.33-14400-training",
        };
        f.write_str(name)
    }
}

impl From<spandsp_sys::t30_indicator_types_e> for T38Indicator {
    fn from(v: spandsp_sys::t30_indicator_types_e) -> Self {
        Self(v)
    }
}

impl From<T38Indicator> for i32 {
    fn from(v: T38Indicator) -> Self {
        v.0 as i32
    }
}

/// T.38 data type, wrapping `t38_data_types_e`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct T38DataType(pub spandsp_sys::t38_data_types_e);

impl T38DataType {
    /// V.21 (300 bps) HDLC signalling.
    pub const V21: Self = Self(spandsp_sys::t38_data_types_e::T38_DATA_V21);
    /// V.27ter at 2400 bps.
    pub const V27TER_2400: Self = Self(spandsp_sys::t38_data_types_e::T38_DATA_V27TER_2400);
    /// V.27ter at 4800 bps.
    pub const V27TER_4800: Self = Self(spandsp_sys::t38_data_types_e::T38_DATA_V27TER_4800);
    /// V.29 at 7200 bps.
    pub const V29_7200: Self = Self(spandsp_sys::t38_data_types_e::T38_DATA_V29_7200);
    /// V.29 at 9600 bps.
    pub const V29_9600: Self = Self(spandsp_sys::t38_data_types_e::T38_DATA_V29_9600);
    /// V.17 at 7200 bps.
    pub const V17_7200: Self = Self(spandsp_sys::t38_data_types_e::T38_DATA_V17_7200);
    /// V.17 at 9600 bps.
    pub const V17_9600: Self = Self(spandsp_sys::t38_data_types_e::T38_DATA_V17_9600);
    /// V.17 at 12000 bps.
    pub const V17_12000: Self = Self(spandsp_sys::t38_data_types_e::T38_DATA_V17_12000);
    /// V.17 at 14400 bps.
    pub const V17_14400: Self = Self(spandsp_sys::t38_data_types_e::T38_DATA_V17_14400);
}

impl fmt::Display for T38DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use spandsp_sys::t38_data_types_e::*;
        let name = match self.0 {
            T38_DATA_NONE => "none",
            T38_DATA_V21 => "V.21",
            T38_DATA_V27TER_2400 => "V.27ter-2400",
            T38_DATA_V27TER_4800 => "V.27ter-4800",
            T38_DATA_V29_7200 => "V.29-7200",
            T38_DATA_V29_9600 => "V.29-9600",
            T38_DATA_V17_7200 => "V.17-7200",
            T38_DATA_V17_9600 => "V.17-9600",
            T38_DATA_V17_12000 => "V.17-12000",
            T38_DATA_V17_14400 => "V.17-14400",
            T38_DATA_V8 => "V.8",
            T38_DATA_V34_PRI_RATE => "V.34-pri-rate",
            T38_DATA_V34_CC_1200 => "V.34-cc-1200",
            T38_DATA_V34_PRI_CH => "V.34-pri-ch",
            T38_DATA_V33_12000 => "V.33-12000",
            T38_DATA_V33_14400 => "V.33-14400",
        };
        f.write_str(name)
    }
}

impl From<spandsp_sys::t38_data_types_e> for T38DataType {
    fn from(v: spandsp_sys::t38_data_types_e) -> Self {
        Self(v)
    }
}

impl From<T38DataType> for i32 {
    fn from(v: T38DataType) -> Self {
        v.0 as i32
    }
}

/// T.38 data field type, wrapping `t38_field_types_e`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct T38FieldType(pub spandsp_sys::t38_field_types_e);

impl T38FieldType {
    /// HDLC data field.
    pub const HDLC_DATA: Self = Self(spandsp_sys::t38_field_types_e::T38_FIELD_HDLC_DATA);
    /// End of HDLC signal.
    pub const HDLC_SIG_END: Self = Self(spandsp_sys::t38_field_types_e::T38_FIELD_HDLC_SIG_END);
    /// HDLC frame with correct FCS.
    pub const HDLC_FCS_OK: Self = Self(spandsp_sys::t38_field_types_e::T38_FIELD_HDLC_FCS_OK);
    /// HDLC frame with bad FCS.
    pub const HDLC_FCS_BAD: Self = Self(spandsp_sys::t38_field_types_e::T38_FIELD_HDLC_FCS_BAD);
    /// T.4 non-ECM image data.
    pub const T4_NON_ECM_DATA: Self =
        Self(spandsp_sys::t38_field_types_e::T38_FIELD_T4_NON_ECM_DATA);
    /// End of T.4 non-ECM signal.
    pub const T4_NON_ECM_SIG_END: Self =
        Self(spandsp_sys::t38_field_types_e::T38_FIELD_T4_NON_ECM_SIG_END);
}

impl fmt::Display for T38FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use spandsp_sys::t38_field_types_e::*;
        let name = match self.0 {
            T38_FIELD_HDLC_DATA => "HDLC-data",
            T38_FIELD_HDLC_SIG_END => "HDLC-sig-end",
            T38_FIELD_HDLC_FCS_OK => "HDLC-FCS-OK",
            T38_FIELD_HDLC_FCS_BAD => "HDLC-FCS-bad",
            T38_FIELD_HDLC_FCS_OK_SIG_END => "HDLC-FCS-OK-sig-end",
            T38_FIELD_HDLC_FCS_BAD_SIG_END => "HDLC-FCS-bad-sig-end",
            T38_FIELD_T4_NON_ECM_DATA => "T4-non-ECM-data",
            T38_FIELD_T4_NON_ECM_SIG_END => "T4-non-ECM-sig-end",
            T38_FIELD_CM_MESSAGE => "CM-message",
            T38_FIELD_JM_MESSAGE => "JM-message",
            T38_FIELD_CI_MESSAGE => "CI-message",
            T38_FIELD_V34RATE => "V34-rate",
        };
        f.write_str(name)
    }
}

impl From<spandsp_sys::t38_field_types_e> for T38FieldType {
    fn from(v: spandsp_sys::t38_field_types_e) -> Self {
        Self(v)
    }
}

impl From<T38FieldType> for i32 {
    fn from(v: T38FieldType) -> Self {
        v.0 as i32
    }
}

bitflags::bitflags! {
    /// T.38 terminal configuration option flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct T38TerminalOptions: i32 {
        /// Disable pacing of T.38 transmissions.
        const NO_PACING = 0x01;
        /// Use regular (non-repeating) indicator packets.
        const REGULAR_INDICATORS = 0x02;
        /// Use 2-second repeating indicator packets.
        const REPEATING_INDICATORS_2S = 0x04;
        /// Suppress indicator packets entirely.
        const NO_INDICATORS = 0x08;
    }
}

impl Default for T38TerminalOptions {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for T38TerminalOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

/// T.38 packet category, wrapping `t38_packet_categories_e`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum T38PacketCategory {
    /// Indicator packet.
    Indicator = spandsp_sys::t38_packet_categories_e_T38_PACKET_CATEGORY_INDICATOR,
    /// Control data packet.
    ControlData = spandsp_sys::t38_packet_categories_e_T38_PACKET_CATEGORY_CONTROL_DATA,
    /// Terminating control data packet.
    ControlDataEnd = spandsp_sys::t38_packet_categories_e_T38_PACKET_CATEGORY_CONTROL_DATA_END,
    /// Image data packet.
    ImageData = spandsp_sys::t38_packet_categories_e_T38_PACKET_CATEGORY_IMAGE_DATA,
    /// Terminating image data packet.
    ImageDataEnd = spandsp_sys::t38_packet_categories_e_T38_PACKET_CATEGORY_IMAGE_DATA_END,
}

/// T.38 protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum T38Version {
    /// T.38 version 0 (original, 1998).
    V0 = 0,
    /// T.38 version 1.
    V1 = 1,
    /// T.38 version 2.
    V2 = 2,
    /// T.38 version 3 (2004 revision with V.34 support).
    V3 = 3,
}

/// T.38 data rate management method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum T38DataRateManagement {
    /// Local TCF: training check performed locally at the gateway.
    LocalTcf = 1,
    /// Transferred TCF: training check passed through to the remote endpoint.
    TransferredTcf = 2,
}

/// T.38 core protocol state wrapping `t38_core_state_t`.
///
/// This is typically obtained via `T38Terminal::get_t38_core_state()` or
/// `T38Gateway::get_t38_core_state()` rather than created directly, but
/// can also be created standalone for custom T.38 implementations.
pub struct T38Core {
    inner: NonNull<spandsp_sys::t38_core_state_t>,
    owned: bool,
}

impl T38Core {
    /// Create a new T.38 core context with raw callback pointers.
    ///
    /// # Safety
    /// All callback function pointers and user_data must remain valid for
    /// the lifetime of this object.
    pub unsafe fn new_raw(
        rx_indicator_handler: spandsp_sys::t38_rx_indicator_handler_t,
        rx_data_handler: spandsp_sys::t38_rx_data_handler_t,
        rx_missing_handler: spandsp_sys::t38_rx_missing_handler_t,
        rx_user_data: *mut std::ffi::c_void,
        tx_packet_handler: spandsp_sys::t38_tx_packet_handler_t,
        tx_packet_user_data: *mut std::ffi::c_void,
    ) -> Result<Self> {
        unsafe {
            let ptr = spandsp_sys::t38_core_init(
                std::ptr::null_mut(),
                rx_indicator_handler,
                rx_data_handler,
                rx_missing_handler,
                rx_user_data,
                tx_packet_handler,
                tx_packet_user_data,
            );
            let inner = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
            Ok(Self { inner, owned: true })
        }
    }

    /// Wrap a non-owned pointer (e.g. from a T38Terminal or T38Gateway).
    ///
    /// # Safety
    /// The pointer must be valid. The object will NOT be freed on drop.
    pub unsafe fn from_raw(ptr: *mut spandsp_sys::t38_core_state_t) -> Result<Self> {
        let inner = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self {
            inner,
            owned: false,
        })
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::t38_core_state_t {
        self.inner.as_ptr()
    }

    /// Restart the T.38 core context.
    pub fn restart(&self) -> Result<()> {
        let rc = unsafe { spandsp_sys::t38_core_restart(self.inner.as_ptr()) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Send an indicator packet.
    ///
    /// Returns the delay (in samples) to allow after sending.
    pub fn send_indicator(&self, indicator: T38Indicator) -> i32 {
        unsafe { spandsp_sys::t38_core_send_indicator(self.inner.as_ptr(), i32::from(indicator)) }
    }

    /// Send a data packet.
    pub fn send_data(
        &self,
        data_type: T38DataType,
        field_type: T38FieldType,
        field: &[u8],
        category: T38PacketCategory,
    ) -> Result<()> {
        let rc = unsafe {
            spandsp_sys::t38_core_send_data(
                self.inner.as_ptr(),
                i32::from(data_type),
                i32::from(field_type),
                field.as_ptr(),
                field.len() as c_int,
                category as c_int,
            )
        };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Process a received IFP packet (unreliable transport like UDPTL/RTP).
    pub fn rx_ifp_packet(&self, buf: &[u8], seq_no: u16) -> Result<()> {
        let rc = unsafe {
            spandsp_sys::t38_core_rx_ifp_packet(
                self.inner.as_ptr(),
                buf.as_ptr(),
                buf.len() as i32,
                seq_no,
            )
        };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Set the T.38 version to emulate.
    pub fn set_t38_version(&self, version: T38Version) {
        unsafe {
            spandsp_sys::t38_set_t38_version(self.inner.as_ptr(), version as c_int);
        }
    }

    /// Set the data rate management method.
    pub fn set_data_rate_management_method(&self, method: T38DataRateManagement) {
        unsafe {
            spandsp_sys::t38_set_data_rate_management_method(self.inner.as_ptr(), method as c_int);
        }
    }

    /// Set redundancy control for a packet category.
    pub fn set_redundancy_control(&self, category: T38PacketCategory, setting: i32) {
        unsafe {
            spandsp_sys::t38_set_redundancy_control(
                self.inner.as_ptr(),
                category as c_int,
                setting as c_int,
            );
        }
    }
}

// SAFETY: T38Core wraps a SpanDSP t38_core_state_t that is only accessed
// through &self/&mut self methods. The underlying C library is not thread-safe,
// but exclusive access can be guaranteed externally (e.g., via tokio::sync::Mutex).
unsafe impl Send for T38Core {}

impl Drop for T38Core {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                spandsp_sys::t38_core_free(self.inner.as_ptr());
            }
        }
    }
}
