# Receive recovery in 0.2.0

Use `FaxState::new_receiver(ReceiveRecovery::PreserveDecodedRows)` for audio,
or unsafe `T38Terminal::new_receiver_raw(policy, handler, user_data)` for T.38.
The policy is captured during construction, before media processing. Existing
`new(false)` / `new_raw(false, ...)` constructors keep preservation disabled.
Existing borrowed T.30 configuration, raw callbacks, modem/compression controls,
ECM controls, and output compression selection continue to work.

```rust
use spandsp::{fax::FaxState, ReceiveRecovery};

let mut fax = FaxState::new_receiver(ReceiveRecovery::PreserveDecodedRows)?;
{
    let t30 = fax.get_t30_state()?;
    t30.set_rx_file("received.tif", -1)?;
    t30.set_ecm_capability(true)?;
    // Optional: select Group 4 TIFF output for a downstream TIFF decoder.
    unsafe {
        spandsp::spandsp_sys::t30_set_supported_output_compressions(
            t30.as_ptr(), spandsp::t4::T4Compression::T6.bits() as i32);
    }
}
// Feed audio using fax.rx()/fax.tx(). On phase E, cancellation, timeout,
// media shutdown, or any error path:
let report = fax.finalize_receive();
// The TIFF is closed now. Inspect completion and output_error separately.
# Ok::<(), spandsp::error::SpanDspError>(())
```

`finalize_receive(&mut self)` is terminal, synchronous and idempotent. It stops
media processing, preserves eligible pending output, finishes TIFF directories,
and closes the file. Repeated calls return the same cached report. It does not
synthesize phase E, call application callbacks, or turn cancellation into success.
The owner cannot restart after finalization; construct a new owner for a new call.
Dropping afterward does not append another page or invoke callbacks. Dropping
without explicit finalization still performs native preservation/closure, but
cannot return a report or output errors.

## Report semantics

- `completion` is the actual phase E code, or `Interrupted { last_status }` if
  phase E has not happened. A zero last status in an interrupted report is **not**
  success. An existing failure is retained across hangup in preservation mode.
- `confirmed_complete_pages` is the original T.30 `pages_rx`, independently of
  the TIFF directory count. Recovery never increments it.
- `pages` inventories written TIFF images in order. Each entry says complete or
  recovered partial, gives the actual stored row count and pixel width, both
  resolutions in **pixels per metre**, detected bad rows, and any omitted tail.
  A protocol-confirmed page can still have damaged image data.
- `output_closed` reports successful TIFF flush and underlying file close (or
  no file needed). `output_error` independently reports open, write/directory,
  allocation, flush, and close errors. On an output error the inventory may be
  incomplete; it is not a claim that every directory is readable. Closure does
  not imply durable storage after power loss.
- `statistics` preserves pre-cleanup native statistics; `get_transfer_statistics`
  remains available through the owner after finalization. Salvage row counts and
  TIFF image counts do not replace native protocol statistics.

## Decodable content and limitations

T.4 1D/2D, T.6, and T.85 recovery retain a **contiguous decoded prefix**. The
unfinished scanline is omitted. The library does not inject zero bits to finish
it, repeat a previous line to conceal damage, or add a blank image. ECM recovery
consumes only consecutive CRC-accepted frames from the pending block, stopping
at the first missing or suspect-length frame; previously committed blocks are
not replayed. Non-ECM T.38 packet gaps and detected decoder damage stop retained
output at the gap. Later rows may depend on missing reference pixels and are
conservatively omitted. `missing_tail` identifies this loss without inventing its
unknown length. Corruption that produces syntactically valid data cannot always
be detected in non-ECM fax.

Other negotiated codecs keep their normal completed-page handling. Partial
recovery for those codecs is not supported: `unsupported_partial_codec` identifies
an omitted unfinished image, and earlier completed images remain available.
There are no native downloads, bundled JPEG/TIFF libraries, or new SSL requirements.
System libtiff and libjpeg are still discovered with pkg-config.

## Ownership and migration

`T30State<'a>` and `T38Core<'a>` borrow their owner. They cannot outlive it or remain
live across `finalize_receive(&mut self)`. Dropping a borrowed handle neither
releases native state nor unregisters callbacks. Do not use a borrowed handle to
free, release, or restart its enclosing native context through raw FFI.

Raw phase B/D/E and packet callback user data belongs to the application. Keep it
alive until callbacks are replaced, the owner is dropped, or receiver finalization
returns. Callbacks must not unwind, reenter the same owner, or run concurrently
with another access to it. Callback storage must be safe on any thread to which
the owner is moved. After finalization, release callback storage freely;
safe media APIs are inert and obtaining a T.38 core handle is rejected. Do not
bypass finalization through cached raw pointers. `FaxState` and `T38Terminal` may
be moved between threads with exclusive ownership; borrowed handles must remain
on the owning thread. `T38Core` no longer implements `Send` because it can borrow
state that the owner could otherwise access concurrently.

The lifetime and exclusive-restart changes are the reason for the 0.2.0 release.
Applications that stored borrowed handles alongside their owner should instead
retrieve short-lived handles when configuring or accessing statistics.

## Validation

`cargo test --workspace` includes 96 real transfer scenarios: PCMA, PCMU, and
T.38; ECM on/off; preservation on/off; interruption before image data, during the
first page, between pages, and during the second page; success, failure after
image completion, transport corruption/loss, and media timeout. TIFF pixels are
compared with generated source images; repeated finalization, callback storage
release, and drop are checked.

Native fault tests add T.4 1D/2D, T.6, and T.85 truncation boundaries, pending ECM
frames with gaps, corruption/concealed-row rejection, a completed page followed by
an undecodable page, and injected output-write and close failures. Compile-fail
doctests verify borrowed-handle ownership and exclusive finalization.

The 96 transport scenarios and native fault tests also pass with the vendored C
code instrumented by AddressSanitizer on macOS arm64.
