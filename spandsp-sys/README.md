# spandsp-sys

Raw FFI bindings for [spandsp](https://github.com/freeswitch/spandsp), auto-generated via bindgen.

## What's included

- Bindings for all public spandsp C APIs
- Vendored spandsp source built via `cc`
- Feature-gated fax modules (T.30, T.38, T.4)

## Features

- **`fax`** (default) — T.30/T.38/T.4 fax support
- **`v32bis`** — V.32bis modem
- **`v34`** — V.34 modem
- **`ssl-fax`** — SSL fax support

## Build dependencies

- C toolchain
- bindgen 0.72+
- **`fax` feature:** libtiff, libjpeg (found via pkg-config)

## Native source

The native submodule uses this repository's `vendor/receive-recovery` branch,
based on upstream FreeSWITCH SpanDSP commit
`797760168945c96e91af55bde9d4edaea2e654f9`, with receive recovery changes.
Initialize source checkouts with `git submodule update --init --recursive`.
Published crates already contain the native sources and need no Git checkout
or native-library downloads during their build.

## License

MIT

## AI DISCLAIMER

Bro this is slop city deluxe. I heavily supervised but it's 2026 get real. **No warranties express or implied** etc etc.
