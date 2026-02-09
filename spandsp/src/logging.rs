//! Safe wrapper around spandsp's logging subsystem.
//!
//! Wraps `logging_state_t` with RAII ownership and provides a safe interface
//! for setting log levels, tags, and custom message handlers.

extern crate spandsp_sys;

use std::ffi::CString;
use std::fmt;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};

/// Log severity levels matching spandsp's SPAN_LOG_* constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum LogLevel {
    None = 0,
    Error = 1,
    Warning = 2,
    ProtocolError = 3,
    ProtocolWarning = 4,
    Flow = 5,
    Flow2 = 6,
    Flow3 = 7,
    Debug = 8,
    Debug2 = 9,
    Debug3 = 10,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            LogLevel::None => "none",
            LogLevel::Error => "error",
            LogLevel::Warning => "warning",
            LogLevel::ProtocolError => "protocol-error",
            LogLevel::ProtocolWarning => "protocol-warning",
            LogLevel::Flow => "flow",
            LogLevel::Flow2 => "flow-2",
            LogLevel::Flow3 => "flow-3",
            LogLevel::Debug => "debug",
            LogLevel::Debug2 => "debug-2",
            LogLevel::Debug3 => "debug-3",
        };
        f.write_str(name)
    }
}

impl From<LogLevel> for i32 {
    fn from(level: LogLevel) -> Self {
        level as i32
    }
}

impl TryFrom<i32> for LogLevel {
    type Error = SpanDspError;

    fn try_from(value: i32) -> std::result::Result<Self, <Self as TryFrom<i32>>::Error> {
        match value {
            0 => Ok(LogLevel::None),
            1 => Ok(LogLevel::Error),
            2 => Ok(LogLevel::Warning),
            3 => Ok(LogLevel::ProtocolError),
            4 => Ok(LogLevel::ProtocolWarning),
            5 => Ok(LogLevel::Flow),
            6 => Ok(LogLevel::Flow2),
            7 => Ok(LogLevel::Flow3),
            8 => Ok(LogLevel::Debug),
            9 => Ok(LogLevel::Debug2),
            10 => Ok(LogLevel::Debug3),
            _ => Err(SpanDspError::InvalidInput(format!(
                "invalid log level: {value}"
            ))),
        }
    }
}

bitflags::bitflags! {
    /// Flags controlling what information is shown in log messages.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct LogShowFlags: i32 {
        /// Show the date in log output.
        const DATE = 0x0100;
        /// Show the sample time in log output.
        const SAMPLE_TIME = 0x0200;
        /// Show the severity level in log output.
        const SEVERITY = 0x0400;
        /// Show the protocol name in log output.
        const PROTOCOL = 0x0800;
        /// Show the variant in log output.
        const VARIANT = 0x1000;
        /// Show the tag in log output.
        const TAG = 0x2000;
        /// Suppress all labelling in log output.
        const SUPPRESS_LABELLING = 0x8000;
    }
}

impl fmt::Display for LogShowFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

type LogHandler = Box<dyn FnMut(LogLevel, &str)>;

/// Trampoline function that converts the C callback into a Rust closure call.
///
/// # Safety
///
/// `user_data` must point to a valid `LogHandler`.
unsafe extern "C" fn message_handler_trampoline(
    user_data: *mut c_void,
    level: c_int,
    text: *const c_char,
) {
    unsafe {
        if user_data.is_null() || text.is_null() {
            return;
        }
        let closure = &mut *(user_data as *mut LogHandler);
        let c_str = std::ffi::CStr::from_ptr(text);
        if let Ok(s) = c_str.to_str() {
            let log_level = LogLevel::try_from(level).unwrap_or(LogLevel::None);
            closure(log_level, s);
        }
    }
}

/// RAII wrapper around `logging_state_t`.
///
/// Created via `LoggingState::new()`, which calls `span_log_init(NULL, ...)`
/// to let spandsp allocate the state internally. Freed on drop via `span_log_free`.
pub struct LoggingState {
    ptr: NonNull<spandsp_sys::logging_state_t>,
    /// Boxed closure kept alive for the lifetime of the handler registration.
    _handler: Option<Box<LogHandler>>,
}

// logging_state_t is not thread-safe. The raw pointer already prevents auto-impl
// of Send and Sync, but we explicitly document the intent here.

impl LoggingState {
    /// Create a new logging state with the given level and tag.
    ///
    /// Passes NULL as the first argument to `span_log_init` so that spandsp
    /// allocates the structure internally.
    pub fn new(level: LogLevel, tag: &str) -> Result<Self> {
        let c_tag = CString::new(tag)
            .map_err(|_| SpanDspError::InvalidInput("tag contains NUL byte".into()))?;
        let ptr = unsafe {
            spandsp_sys::span_log_init(std::ptr::null_mut(), level as c_int, c_tag.as_ptr())
        };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self {
            ptr,
            _handler: None,
        })
    }

    /// Wrap an existing non-null pointer to a `logging_state_t`.
    ///
    /// # Safety
    ///
    /// The caller must ensure the pointer is valid and that this wrapper will
    /// **not** free it on drop. This constructor is intended for borrowed
    /// references obtained from other spandsp objects (e.g. `fax_get_logging_state`).
    /// To prevent double-free, prefer using `as_ptr()` on the parent object instead.
    pub unsafe fn from_ptr_borrowed(ptr: NonNull<spandsp_sys::logging_state_t>) -> Self {
        // NOTE: We store it but Drop will call span_log_free. Only use this
        // for states that were allocated via span_log_init(NULL,...).
        Self {
            ptr,
            _handler: None,
        }
    }

    /// Return the raw pointer to the underlying logging state.
    pub fn as_ptr(&self) -> *mut spandsp_sys::logging_state_t {
        self.ptr.as_ptr()
    }

    /// Set the log level.
    pub fn set_level(&mut self, level: LogLevel) {
        unsafe {
            spandsp_sys::span_log_set_level(self.ptr.as_ptr(), level as c_int);
        }
    }

    /// Set the log level with additional show flags combined.
    ///
    /// The level occupies the low 8 bits and the flags occupy the upper bits.
    pub fn set_level_with_flags(&mut self, level: LogLevel, flags: LogShowFlags) {
        let combined = (level as i32) | flags.bits();
        unsafe {
            spandsp_sys::span_log_set_level(self.ptr.as_ptr(), combined as c_int);
        }
    }

    /// Set the log tag.
    pub fn set_tag(&mut self, tag: &str) -> Result<()> {
        let c_tag = CString::new(tag)
            .map_err(|_| SpanDspError::InvalidInput("tag contains NUL byte".into()))?;
        unsafe {
            spandsp_sys::span_log_set_tag(self.ptr.as_ptr(), c_tag.as_ptr());
        }
        Ok(())
    }

    /// Set the log protocol string.
    pub fn set_protocol(&mut self, protocol: &str) -> Result<()> {
        let c_proto = CString::new(protocol)
            .map_err(|_| SpanDspError::InvalidInput("protocol contains NUL byte".into()))?;
        unsafe {
            spandsp_sys::span_log_set_protocol(self.ptr.as_ptr(), c_proto.as_ptr());
        }
        Ok(())
    }

    /// Set the sample rate for time-stamped log messages.
    pub fn set_sample_rate(&mut self, samples_per_second: i32) {
        unsafe {
            spandsp_sys::span_log_set_sample_rate(self.ptr.as_ptr(), samples_per_second as c_int);
        }
    }

    /// Set a custom message handler closure.
    ///
    /// The closure receives `(level, message_text)` for each log message.
    /// The closure is kept alive for the lifetime of this `LoggingState` or
    /// until a new handler is set.
    pub fn set_message_handler<F>(&mut self, handler: F)
    where
        F: FnMut(LogLevel, &str) + 'static,
    {
        let boxed: Box<LogHandler> = Box::new(Box::new(handler));
        let user_data = &*boxed as *const LogHandler as *mut c_void;
        unsafe {
            spandsp_sys::span_log_set_message_handler(
                self.ptr.as_ptr(),
                Some(message_handler_trampoline),
                user_data,
            );
        }
        self._handler = Some(boxed);
    }
}

impl Drop for LoggingState {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::span_log_free(self.ptr.as_ptr());
        }
    }
}

/// Set the global (default) message handler for all spandsp logging.
///
/// # Safety
///
/// The provided function pointer and user data must remain valid for the
/// lifetime of the program or until replaced. Prefer using per-instance
/// `LoggingState::set_message_handler` when possible.
pub unsafe fn set_global_message_handler(
    handler: spandsp_sys::message_handler_func_t,
    user_data: *mut c_void,
) {
    unsafe {
        spandsp_sys::span_set_message_handler(handler, user_data);
    }
}
