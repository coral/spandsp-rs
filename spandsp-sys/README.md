# spandsp-sys

Raw FFI bindings for [spandsp](https://github.com/freeswitch/spandsp), auto-generated via bindgen.

## What's included

- Bindings for public spandsp C APIs (`v150` APIs require opt-in)
- Vendored spandsp source built via `cc`
- Feature-gated fax modules (T.30, T.38, T.4)

## Features

- **`fax`** (default) — T.30/T.38/T.4 fax support
- **`v32bis`** — V.32bis modem
- **`v34`** — V.34 modem
- **`v150`** (off by default) — V.150.1 modem relay, SPRT, and SSE; enables GPL-2.0-only native code and bindings
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

The Rust bindings and build glue are MIT licensed. This does not relicense the
vendored C code. SpanDSP's [COPYING](vendor/COPYING) describes LGPL 2.1 library
licensing and GPL 2 supporting code; individual source notices also apply.

In particular, `vendor/src/sprt.c`, `vendor/src/v150_1.c`, and
`vendor/src/v150_1_sse.c`, along with their associated headers, carry
**GPL-2.0-only** notices. These three C files are compiled into the native static
archive only when **`v150`** is enabled. Their public headers are also omitted
from bindgen input when the feature is disabled, so their functions and types
are unavailable in Rust. Default builds (including `fax`) exclude them.

To use these APIs, enable `features = ["v150"]` on either `spandsp-sys` or
`spandsp`. This is now required for callers that previously used the raw
`sprt_*` or `v150_1_*` APIs without a feature.

To keep these modules out of the build, leave `v150` disabled and avoid
`--all-features`. [Cargo features are additive](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification):
another dependency can enable `spandsp-sys/v150` for the shared build. Inspect
the resolved features with `cargo tree -e features -i spandsp-sys`.

This feature controls compilation and generated bindings, **not source
packaging**: the GPL-marked files remain in the repository and published crate
archive. Other vendored source and build tools retain their own licenses;
disabling `v150` is not a claim that the entire package is MIT-only or GPL-free.

## AI DISCLAIMER

Bro this is slop city deluxe. I heavily supervised but it's 2026 get real. **No warranties express or implied** etc etc.
