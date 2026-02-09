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

## License

MIT
