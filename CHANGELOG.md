# Changelog

## 0.2.0 — 2026-09-07

- Add opt-in `ReceiveRecovery::PreserveDecodedRows` to audio fax and T.38
  receivers, with recovery before native error/hangup/release cleanup.
- Add synchronous, terminal, idempotent `finalize_receive` and typed reports
  separating protocol completion and confirmations from TIFF output and errors.
- Preserve original statistics and failure codes. Recover consecutive decoded
  rows and pending ECM frames without inserting replacement image content.
- Tie borrowed T.30/core handles to owner lifetimes; require exclusive access for
  receiver restart/finalization. Borrowed T.38 core handles no longer implement Send.
- Rebuild native sources when vendored files change. System library discovery and
  default features are unchanged.

See [receive recovery](spandsp/RECEIVE_RECOVERY.md) for migration, ownership,
codec coverage, and test details.
