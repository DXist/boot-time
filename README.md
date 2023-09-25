# Boot time

[![Crates.io](https://img.shields.io/crates/v/boot-time)](https://crates.io/crates/boot-time)
[![Docs.rs](https://img.shields.io/docsrs/boot-time/latest)](https://docs.rs/boot-time)
[![License](https://img.shields.io/crates/l/boot-time)](https://raw.githubusercontent.com/DXist/boot-time/main/LICENSE)

This library reimplements `std::time::Instant` to use suspend-aware monotonic time if target system supports it.
Otherwise is uses monotonic time.
