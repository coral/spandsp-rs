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
- **`v150` feature (off by default):** raw V.150.1, SPRT, and SSE APIs; enables GPL-2.0-only native code

## Dependencies

- C toolchain (cc)
- spandsp C library (linked via pkg-config or built from vendored source in `spandsp-sys/vendor`)

## License

The Rust bindings and wrappers are MIT licensed. Vendored SpanDSP has separate
license notices; see [native licensing and the `v150` feature](spandsp-sys/README.md#license).
Default builds exclude `sprt.c`, `v150_1.c`, and `v150_1_sse.c` and their bindings.
Their GPL-2.0-only code is enabled only by the opt-in `v150` feature.

## AI DISCLAIMER

Bro this is slop city deluxe. I heavily supervised but it's 2026 get real. **No warranties express or implied** etc etc.
