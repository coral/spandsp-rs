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

## License

The Rust wrappers are MIT licensed. Vendored SpanDSP has separate license notices;
see [native licensing](https://github.com/coral/spandsp-rs/tree/master/spandsp-sys#license).

The optional **`v150`** feature forwards to `spandsp-sys/v150`, compiling the
GPL-2.0-only `sprt.c`, `v150_1.c`, and `v150_1_sse.c` and exposing their raw APIs
through `spandsp::spandsp_sys`. It is off by default, including with `fax` enabled.
Existing users of these raw APIs must now enable `v150` explicitly.
`--all-features` or another dependency enabling `spandsp-sys/v150` also enables
this code. Disabling the feature excludes compilation and bindings, but the
vendored files remain in the `spandsp-sys` source package.

## AI DISCLAIMER

Bro this is slop city deluxe. I heavily supervised but it's 2026 get real. **No warranties express or implied** etc etc.

## Receive recovery

Version 0.2.0 adds opt-in preservation of decoded rows and explicit receiver
finalization. See [the API and migration guide](RECEIVE_RECOVERY.md).
