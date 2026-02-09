//! Safe wrapper around the T.30 FAX protocol engine.

use std::ffi::CString;
use std::fmt;
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError, T30Error};

bitflags::bitflags! {
    /// Supported modem types for T.30 negotiation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct T30ModemSupport: i32 {
        /// V.27ter (2400/4800 bps).
        const V27TER = 0x01;
        /// V.29 (7200/9600 bps).
        const V29 = 0x02;
        /// V.17 (7200-14400 bps).
        const V17 = 0x04;
        /// V.34 half-duplex.
        const V34HDX = 0x08;
        /// Internet-Aware FAX (IAF).
        const IAF = 0x10;
    }
}

impl Default for T30ModemSupport {
    /// Default: V.27ter + V.29 + V.17 (standard fax modems).
    fn default() -> Self {
        Self::V27TER | Self::V29 | Self::V17
    }
}

impl fmt::Display for T30ModemSupport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

/// T.30 FAX protocol state machine.
///
/// This is typically obtained via `FaxState::get_t30_state()` or
/// `T38Terminal::get_t30_state()` rather than created directly.
pub struct T30State {
    inner: NonNull<spandsp_sys::t30_state_t>,
    owned: bool,
}

impl T30State {
    /// Wrap an existing pointer obtained from another spandsp object.
    ///
    /// # Safety
    /// The pointer must be valid. `owned` controls whether `t30_free` is called on drop.
    pub unsafe fn from_raw(ptr: *mut spandsp_sys::t30_state_t, owned: bool) -> Result<Self> {
        let inner = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { inner, owned })
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::t30_state_t {
        self.inner.as_ptr()
    }

    /// Set the file to transmit.
    pub fn set_tx_file(&self, file: &str, start_page: i32, stop_page: i32) -> Result<()> {
        let c_file = CString::new(file)
            .map_err(|_| SpanDspError::InvalidInput("file path contains NUL".into()))?;
        unsafe {
            spandsp_sys::t30_set_tx_file(
                self.inner.as_ptr(),
                c_file.as_ptr(),
                start_page,
                stop_page,
            );
        }
        Ok(())
    }

    /// Set the file to receive into.
    pub fn set_rx_file(&self, file: &str, stop_page: i32) -> Result<()> {
        let c_file = CString::new(file)
            .map_err(|_| SpanDspError::InvalidInput("file path contains NUL".into()))?;
        unsafe {
            spandsp_sys::t30_set_rx_file(self.inner.as_ptr(), c_file.as_ptr(), stop_page);
        }
        Ok(())
    }

    /// Set supported modems for T.30 negotiation.
    pub fn set_supported_modems(&self, modems: T30ModemSupport) -> Result<()> {
        let rc =
            unsafe { spandsp_sys::t30_set_supported_modems(self.inner.as_ptr(), modems.bits()) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Enable or disable ECM.
    pub fn set_ecm_capability(&self, enabled: bool) -> Result<()> {
        let rc = unsafe { spandsp_sys::t30_set_ecm_capability(self.inner.as_ptr(), enabled) };
        if rc != 0 {
            return Err(SpanDspError::ErrorCode(rc));
        }
        Ok(())
    }

    /// Get the current transfer statistics.
    pub fn get_transfer_statistics(&self) -> spandsp_sys::t30_stats_t {
        let mut stats = unsafe { std::mem::zeroed::<spandsp_sys::t30_stats_t>() };
        unsafe {
            spandsp_sys::t30_get_transfer_statistics(self.inner.as_ptr(), &mut stats);
        }
        stats
    }

    /// Set the T.30 phase B handler (called at start of document exchange).
    ///
    /// # Safety
    /// The callback and user_data must remain valid for the lifetime of this state.
    pub unsafe fn set_phase_b_handler_raw(
        &self,
        handler: spandsp_sys::t30_phase_b_handler_t,
        user_data: *mut std::ffi::c_void,
    ) {
        unsafe {
            spandsp_sys::t30_set_phase_b_handler(self.inner.as_ptr(), handler, user_data);
        }
    }

    /// Set the T.30 phase D handler (called at end of each page).
    ///
    /// # Safety
    /// The callback and user_data must remain valid for the lifetime of this state.
    pub unsafe fn set_phase_d_handler_raw(
        &self,
        handler: spandsp_sys::t30_phase_d_handler_t,
        user_data: *mut std::ffi::c_void,
    ) {
        unsafe {
            spandsp_sys::t30_set_phase_d_handler(self.inner.as_ptr(), handler, user_data);
        }
    }

    /// Set the T.30 phase E handler (called at completion of fax session).
    ///
    /// # Safety
    /// The callback and user_data must remain valid for the lifetime of this state.
    pub unsafe fn set_phase_e_handler_raw(
        &self,
        handler: spandsp_sys::t30_phase_e_handler_t,
        user_data: *mut std::ffi::c_void,
    ) {
        unsafe {
            spandsp_sys::t30_set_phase_e_handler(self.inner.as_ptr(), handler, user_data);
        }
    }

    /// Check if the T.30 call is still active.
    pub fn call_active(&self) -> bool {
        unsafe { spandsp_sys::t30_call_active(self.inner.as_ptr()) != 0 }
    }

    /// Convert a T.30 completion code to a `T30Error`.
    ///
    /// Returns `None` if the code does not correspond to a known `t30_err_e`
    /// discriminant.
    pub fn completion_code(code: i32) -> Option<T30Error> {
        use spandsp_sys::t30_err_e::*;
        let raw = match code as u32 {
            0 => T30_ERR_OK,
            1 => T30_ERR_CEDTONE,
            2 => T30_ERR_T0_EXPIRED,
            3 => T30_ERR_T1_EXPIRED,
            4 => T30_ERR_T3_EXPIRED,
            5 => T30_ERR_HDLC_CARRIER,
            6 => T30_ERR_CANNOT_TRAIN,
            7 => T30_ERR_OPER_INT_FAIL,
            8 => T30_ERR_INCOMPATIBLE,
            9 => T30_ERR_RX_INCAPABLE,
            10 => T30_ERR_TX_INCAPABLE,
            11 => T30_ERR_NORESSUPPORT,
            12 => T30_ERR_NOSIZESUPPORT,
            13 => T30_ERR_UNEXPECTED,
            14 => T30_ERR_TX_BADDCS,
            15 => T30_ERR_TX_BADPG,
            16 => T30_ERR_TX_ECMPHD,
            17 => T30_ERR_TX_GOTDCN,
            18 => T30_ERR_TX_INVALRSP,
            19 => T30_ERR_TX_NODIS,
            20 => T30_ERR_TX_PHBDEAD,
            21 => T30_ERR_TX_PHDDEAD,
            22 => T30_ERR_TX_T5EXP,
            23 => T30_ERR_RX_ECMPHD,
            24 => T30_ERR_RX_GOTDCS,
            25 => T30_ERR_RX_INVALCMD,
            26 => T30_ERR_RX_NOCARRIER,
            27 => T30_ERR_RX_NOEOL,
            28 => T30_ERR_RX_NOFAX,
            29 => T30_ERR_RX_T2EXPDCN,
            30 => T30_ERR_RX_T2EXPD,
            31 => T30_ERR_RX_T2EXPFAX,
            32 => T30_ERR_RX_T2EXPMPS,
            33 => T30_ERR_RX_T2EXPRR,
            34 => T30_ERR_RX_T2EXP,
            35 => T30_ERR_RX_DCNWHY,
            36 => T30_ERR_RX_DCNDATA,
            37 => T30_ERR_RX_DCNFAX,
            38 => T30_ERR_RX_DCNPHD,
            39 => T30_ERR_RX_DCNRRD,
            40 => T30_ERR_RX_DCNNORTN,
            41 => T30_ERR_FILEERROR,
            42 => T30_ERR_NOPAGE,
            43 => T30_ERR_BADTIFF,
            44 => T30_ERR_BADPAGE,
            45 => T30_ERR_BADTAG,
            46 => T30_ERR_BADTIFFHDR,
            47 => T30_ERR_NOMEM,
            48 => T30_ERR_RETRYDCN,
            49 => T30_ERR_CALLDROPPED,
            50 => T30_ERR_NOPOLL,
            51 => T30_ERR_IDENT_UNACCEPTABLE,
            52 => T30_ERR_SUB_UNACCEPTABLE,
            53 => T30_ERR_SEP_UNACCEPTABLE,
            54 => T30_ERR_PSA_UNACCEPTABLE,
            55 => T30_ERR_SID_UNACCEPTABLE,
            56 => T30_ERR_PWD_UNACCEPTABLE,
            57 => T30_ERR_TSA_UNACCEPTABLE,
            58 => T30_ERR_IRA_UNACCEPTABLE,
            59 => T30_ERR_CIA_UNACCEPTABLE,
            60 => T30_ERR_ISP_UNACCEPTABLE,
            61 => T30_ERR_CSA_UNACCEPTABLE,
            _ => return None,
        };
        Some(T30Error::from(raw))
    }
}

impl Drop for T30State {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                spandsp_sys::t30_free(self.inner.as_ptr());
            }
        }
    }
}
