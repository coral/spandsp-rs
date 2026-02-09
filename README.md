# spandsp-rs

Rust bindings for [spandsp](https://github.com/freeswitch/spandsp), a DSP library for telephony.

## Crates

- **`spandsp-sys`** — raw FFI bindings (auto-generated via bindgen)
- **`spandsp`** — safe, idiomatic Rust wrappers

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

- C toolchain (cc)
- spandsp C library (linked via pkg-config or built from vendored source in `spandsp-sys/vendor`)

## License

MIT

## AI DISCLAIMER

Bro this is slop city deluxe. I heavily supervised but it's 2026 get real. **No warranties express or implied** etc etc.