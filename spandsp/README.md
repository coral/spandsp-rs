# spandsp

Safe Rust wrappers for [spandsp](https://github.com/freeswitch/spandsp).

## What's wrapped

- G.711, G.722, G.726 codecs
- DTMF generation & detection
- HDLC framing / deframing
- Tone generation & Goertzel detection
- Echo cancellation
- Power metering
- Logging
- **`fax` feature (default):** T.30, T.38 core/terminal/gateway, T.4 encode/decode, fax modems

## Dependencies

- `spandsp-sys` (raw FFI bindings)
- `bitflags` 2
- `thiserror` 2

## License

MIT
