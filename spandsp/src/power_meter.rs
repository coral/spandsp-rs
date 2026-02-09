//! Safe wrapper around spandsp's power meter.
//!
//! Wraps `power_meter_t` for measuring the power level of an audio signal.

extern crate spandsp_sys;

use std::os::raw::c_int;
use std::ptr::NonNull;

use crate::error::{Result, SpanDspError};

/// RAII wrapper around `power_meter_t`.
///
/// Created via `PowerMeter::new()`, which calls `power_meter_init(NULL, shift)`.
/// Freed on drop via `power_meter_free`.
pub struct PowerMeter {
    ptr: NonNull<spandsp_sys::power_meter_t>,
}

impl PowerMeter {
    /// Create a new power meter.
    ///
    /// `shift` controls the damping factor of the IIR filter. Larger values
    /// give a slower (more smoothed) response.
    pub fn new(shift: i32) -> Result<Self> {
        let ptr = unsafe { spandsp_sys::power_meter_init(std::ptr::null_mut(), shift as c_int) };
        let ptr = NonNull::new(ptr).ok_or(SpanDspError::InitFailed)?;
        Ok(Self { ptr })
    }

    /// Update the power meter with a single audio sample.
    ///
    /// Returns the current (raw) power meter reading.
    pub fn update(&mut self, amp: i16) -> i32 {
        unsafe { spandsp_sys::power_meter_update(self.ptr.as_ptr(), amp) }
    }

    /// Get the current power meter reading (raw integer value).
    pub fn current(&self) -> i32 {
        unsafe { spandsp_sys::power_meter_current(self.ptr.as_ptr()) }
    }

    /// Get the current power meter reading in dBm0.
    pub fn current_dbm0(&self) -> f32 {
        unsafe { spandsp_sys::power_meter_current_dbm0(self.ptr.as_ptr()) }
    }

    /// Get the current power meter reading in dBOv.
    pub fn current_dbov(&self) -> f32 {
        unsafe { spandsp_sys::power_meter_current_dbov(self.ptr.as_ptr()) }
    }

    /// Change the damping factor.
    pub fn set_damping(&mut self, shift: i32) {
        unsafe {
            spandsp_sys::power_meter_damping(self.ptr.as_ptr(), shift as c_int);
        }
    }

    /// Return the raw pointer.
    pub fn as_ptr(&self) -> *mut spandsp_sys::power_meter_t {
        self.ptr.as_ptr()
    }
}

impl Drop for PowerMeter {
    fn drop(&mut self) {
        unsafe {
            spandsp_sys::power_meter_free(self.ptr.as_ptr());
        }
    }
}

/// Convert a dBm0 level to the equivalent raw power meter reading.
pub fn level_dbm0(level: f32) -> i32 {
    unsafe { spandsp_sys::power_meter_level_dbm0(level) }
}

/// Convert a dBOv level to the equivalent raw power meter reading.
pub fn level_dbov(level: f32) -> i32 {
    unsafe { spandsp_sys::power_meter_level_dbov(level) }
}
