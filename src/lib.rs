//! Temporal quantification that takes into account the time a system spent suspended.
//!
//! Note: Some systems like FreeBSD and AIX don't support CLOCK_BOOTIME. For compatibility CLOCK_MONOTONIC is used as a fallback.
//!
//! For unsupported platforms `std::time::Instant` is just reexported.
//!
//! # Examples
//!
//! Using [`Instant`] to calculate how long a function took to run:
//!
//! ```ignore (incomplete)
//! let now = Instant::now();
//!
//! // Calling a slow function, it may take a while
//! slow_function();
//!
//! let elapsed_time = now.elapsed();
//! println!("Running slow_function() took {} seconds.", elapsed_time.as_secs());
//! ```

pub use core::time::Duration;

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        mod time;
        mod sys;
        mod sys_common;

        pub use self::time::Instant;
    } else {
        pub use std::time::Instant;
    }
}

#[cfg(test)]
mod tests;
