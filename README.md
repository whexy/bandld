# BAND-ld

BAND-ld is a straightforward `ld` wrapper designed to ensure that when the `--wrap=symbol` option is used and `__wrap_symbol` is not defined, the target can still be correctly linked. It does this by automatically providing a default implementation of the `__wrap_symbol` function.

## Usage

1. Build `bandld` using Cargo:
    cargo build --release

2. Rename your existing ld to ld-orig:
    `mv /usr/bin/ld /usr/bin/ld-orig`

3. Copy the built bandld to replace the original ld:
    `cp /target/release/bandld /usr/bin/ld`

## Debugging
To view detailed logs, check the files located at `/tmp/wrap_symbols_*`.
