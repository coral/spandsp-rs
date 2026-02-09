//! Error types for the spandsp crate.

/// Errors that can occur when using spandsp wrappers.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SpanDspError {
    /// Initialization of a spandsp resource failed (NULL returned).
    #[error("initialization failed")]
    InitFailed,
    /// A spandsp function returned a numeric error code.
    #[error("error code: {0}")]
    ErrorCode(i32),
    /// An invalid input was provided to a wrapper function.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// A T.30 FAX protocol error.
    #[cfg(feature = "fax")]
    #[error("T.30 error: {0}")]
    T30(#[from] T30Error),
}

impl From<i32> for SpanDspError {
    fn from(code: i32) -> Self {
        SpanDspError::ErrorCode(code)
    }
}

/// A convenience Result type for spandsp operations.
pub type Result<T> = std::result::Result<T, SpanDspError>;

// ---------------------------------------------------------------------------
// T.30 Error
// ---------------------------------------------------------------------------

/// T.30 FAX protocol error, wrapping the `t30_err_e` enum from spandsp.
#[cfg(feature = "fax")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("{}", .0.description())]
pub struct T30Error(pub spandsp_sys::t30_err_e);

#[cfg(feature = "fax")]
impl T30Error {
    /// The operation completed successfully.
    pub const OK: Self = Self(spandsp_sys::t30_err_e::T30_ERR_OK);

    /// Returns `true` if this represents a successful completion.
    pub fn is_ok(self) -> bool {
        self.0 == spandsp_sys::t30_err_e::T30_ERR_OK
    }

    /// Returns the raw `t30_err_e` enum value.
    pub fn raw(self) -> spandsp_sys::t30_err_e {
        self.0
    }
}

#[cfg(feature = "fax")]
trait T30ErrDescription {
    fn description(&self) -> &'static str;
}

#[cfg(feature = "fax")]
impl T30ErrDescription for spandsp_sys::t30_err_e {
    fn description(&self) -> &'static str {
        use spandsp_sys::t30_err_e::*;
        match *self {
            T30_ERR_OK => "OK",
            T30_ERR_CEDTONE => "CED tone detected",
            T30_ERR_T0_EXPIRED => "T0 timer expired",
            T30_ERR_T1_EXPIRED => "T1 timer expired",
            T30_ERR_T3_EXPIRED => "T3 timer expired",
            T30_ERR_HDLC_CARRIER => "HDLC carrier lost",
            T30_ERR_CANNOT_TRAIN => "cannot train modem",
            T30_ERR_OPER_INT_FAIL => "operator intervention failed",
            T30_ERR_INCOMPATIBLE => "incompatible remote capabilities",
            T30_ERR_RX_INCAPABLE => "remote cannot receive",
            T30_ERR_TX_INCAPABLE => "remote cannot transmit",
            T30_ERR_NORESSUPPORT => "resolution not supported",
            T30_ERR_NOSIZESUPPORT => "image size not supported",
            T30_ERR_UNEXPECTED => "unexpected message received",
            T30_ERR_TX_BADDCS => "bad DCS received during transmit",
            T30_ERR_TX_BADPG => "bad page received during transmit",
            T30_ERR_TX_ECMPHD => "ECM page error during transmit",
            T30_ERR_TX_GOTDCN => "DCN received during transmit",
            T30_ERR_TX_INVALRSP => "invalid response during transmit",
            T30_ERR_TX_NODIS => "no DIS received",
            T30_ERR_TX_PHBDEAD => "phase B dead during transmit",
            T30_ERR_TX_PHDDEAD => "phase D dead during transmit",
            T30_ERR_TX_T5EXP => "T5 timer expired during transmit",
            T30_ERR_RX_ECMPHD => "ECM page error during receive",
            T30_ERR_RX_GOTDCS => "unexpected DCS during receive",
            T30_ERR_RX_INVALCMD => "invalid command during receive",
            T30_ERR_RX_NOCARRIER => "no carrier during receive",
            T30_ERR_RX_NOEOL => "no end-of-line during receive",
            T30_ERR_RX_NOFAX => "no fax detected",
            T30_ERR_RX_T2EXPDCN => "T2 expired, DCN received",
            T30_ERR_RX_T2EXPD => "T2 expired in phase D",
            T30_ERR_RX_T2EXPFAX => "T2 expired waiting for fax",
            T30_ERR_RX_T2EXPMPS => "T2 expired waiting for MPS",
            T30_ERR_RX_T2EXPRR => "T2 expired waiting for RR",
            T30_ERR_RX_T2EXP => "T2 expired",
            T30_ERR_RX_DCNWHY => "DCN received unexpectedly",
            T30_ERR_RX_DCNDATA => "DCN received during data",
            T30_ERR_RX_DCNFAX => "DCN received during fax",
            T30_ERR_RX_DCNPHD => "DCN received during phase D",
            T30_ERR_RX_DCNRRD => "DCN received during RR/D",
            T30_ERR_RX_DCNNORTN => "DCN received, no retrain",
            T30_ERR_FILEERROR => "file I/O error",
            T30_ERR_NOPAGE => "no page to send",
            T30_ERR_BADTIFF => "bad TIFF file",
            T30_ERR_BADPAGE => "bad page",
            T30_ERR_BADTAG => "bad TIFF tag",
            T30_ERR_BADTIFFHDR => "bad TIFF header",
            T30_ERR_NOMEM => "out of memory",
            _ => "unknown T.30 error",
        }
    }
}

#[cfg(feature = "fax")]
impl From<spandsp_sys::t30_err_e> for T30Error {
    fn from(e: spandsp_sys::t30_err_e) -> Self {
        Self(e)
    }
}

#[cfg(feature = "fax")]
impl From<T30Error> for spandsp_sys::t30_err_e {
    fn from(e: T30Error) -> Self {
        e.0
    }
}
