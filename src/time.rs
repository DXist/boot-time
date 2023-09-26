//! Reimplementation of `std::time::Instant` for supported platforms
use core::time::Duration;
use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::sys;

/// A measurement of a suspend-aware monotonically nondecreasing clock.
/// Opaque and useful only with [`Duration`].
///
/// Instants are always guaranteed, barring [platform bugs], to be no less than any previously
/// measured instant when created, and are often useful for tasks such as measuring
/// benchmarks or timing how long an operation takes.
///
/// Note, however, that instants are **not** guaranteed to be **steady**. In other
/// words, each tick of the underlying clock might not be the same length (e.g.
/// some seconds may be longer than others). An instant may jump forwards or
/// experience time dilation (slow down or speed up), but it will never go
/// backwards.
///
/// Instants are opaque types that can only be compared to one another. There is
/// no method to get "the number of seconds" from an instant. Instead, it only
/// allows measuring the duration between two instants (or comparing two
/// instants).
///
/// The size of an `Instant` struct may vary depending on the target operating
/// system.
///
/// Example:
///
/// ```no_run
/// use boot_time::{Duration, Instant};
/// use std::thread::sleep;
///
/// fn main() {
///    let now = Instant::now();
///
///    // we sleep for 2 seconds
///    sleep(Duration::new(2, 0));
///    // it prints '2'
///    println!("{}", now.elapsed().as_secs());
/// }
/// ```
///
/// [platform bugs]: Instant#monotonicity
///
/// # OS-specific behaviors
///
/// An `Instant` is a wrapper around system-specific types and it may behave
/// differently depending on the underlying operating system. For example,
/// the following snippet is fine on Linux but panics on macOS:
///
/// ```no_run
/// use std::time::{Instant, Duration};
///
/// let now = Instant::now();
/// let max_seconds = u64::MAX / 1_000_000_000;
/// let duration = Duration::new(max_seconds, 0);
/// println!("{:?}", now + duration);
/// ```
///
/// # Underlying System calls
///
/// The following system calls are [currently] being used by `now()` to find out
/// the current time:
///
/// |  Platform                         |               System call                                            |
/// |-----------------------------------|----------------------------------------------------------------------|
/// | UNIX (Linux/L4Re/Emscripten/      |                                                                      |
/// |       Android/Fuchsia/openBSD)    | [clock_gettime (Monotonic Clock with CLOCK_BOOTTIME source)]         |
/// | UNIX (other)                      | [clock_gettime (Monotonic Clock with CLOCK_MONOTONIC source)]        |
/// | Darwin                            | [mach_continuous_time]                                               |
/// | Windows                           | [QueryPerformanceCounter]                                                                    |
///
/// [currently]: crate::io#platform-specific-behavior
/// [QueryPerformanceCounter]: https://docs.microsoft.com/en-us/windows/win32/api/profileapi/nf-profileapi-queryperformancecounter
/// [clock_gettime (Monotonic Clock)]: https://linux.die.net/man/3/clock_gettime
/// FreeBSD doesn't take into account suspended time in CLOCK_BOOTIME. So CLOCK_MONOTONIC is used.
/// [mach_absolute_time]: https://developer.apple.com/documentation/kernel/1646199-mach_continuous_time
///
/// **Disclaimer:** These system calls might change over time.
///
/// > Note: mathematical operations like [`add`] may panic if the underlying
/// > structure cannot represent the new point in time.
///
/// [`add`]: Instant::add
///
/// ## Monotonicity
///
/// On all platforms `Instant` will try to use an OS API that guarantees monotonic behavior
/// if available, which is the case for all [tier 1] platforms.
/// In practice such guarantees are – under rare circumstances – broken by hardware, virtualization
/// or operating system bugs. To work around these bugs and platforms not offering monotonic clocks
/// [`duration_since`], [`elapsed`] and [`sub`] saturate to zero. In older Rust versions this
/// lead to a panic instead. [`checked_duration_since`] can be used to detect and handle situations
/// where monotonicity is violated, or `Instant`s are subtracted in the wrong order.
///
/// This workaround obscures programming errors where earlier and later instants are accidentally
/// swapped. For this reason future rust versions may reintroduce panics.
///
/// [tier 1]: https://doc.rust-lang.org/rustc/platform-support.html
/// [`duration_since`]: Instant::duration_since
/// [`elapsed`]: Instant::elapsed
/// [`sub`]: Instant::sub
/// [`checked_duration_since`]: Instant::checked_duration_since
///
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(sys::Instant);

impl Instant {
    /// Returns an instant corresponding to "now".
    ///
    /// # Examples
    ///
    /// ```
    /// use boot_time::Instant;
    ///
    /// let now = Instant::now();
    /// ```
    #[must_use]
    pub fn now() -> Instant {
        Instant(sys::Instant::now())
    }

    /// Returns the amount of time elapsed from another instant to this one,
    /// or zero duration if that instant is later than this one.
    ///
    /// # Panics
    ///
    /// Previous rust versions panicked when `earlier` was later than `self`. Currently this
    /// method saturates. Future versions may reintroduce the panic in some circumstances.
    /// See [Monotonicity].
    ///
    /// [Monotonicity]: Instant#monotonicity
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use boot_time::{Duration, Instant};
    /// use std::thread::sleep;
    ///
    /// let now = Instant::now();
    /// sleep(Duration::new(1, 0));
    /// let new_now = Instant::now();
    /// println!("{:?}", new_now.duration_since(now));
    /// println!("{:?}", now.duration_since(new_now)); // 0ns
    /// ```
    #[must_use]
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.checked_duration_since(earlier).unwrap_or_default()
    }

    /// Returns the amount of time elapsed from another instant to this one,
    /// or None if that instant is later than this one.
    ///
    /// Due to [monotonicity bugs], even under correct logical ordering of the passed `Instant`s,
    /// this method can return `None`.
    ///
    /// [monotonicity bugs]: Instant#monotonicity
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use boot_time::{Duration, Instant};
    /// use std::thread::sleep;
    ///
    /// let now = Instant::now();
    /// sleep(Duration::new(1, 0));
    /// let new_now = Instant::now();
    /// println!("{:?}", new_now.checked_duration_since(now));
    /// println!("{:?}", now.checked_duration_since(new_now)); // None
    /// ```
    #[must_use]
    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        self.0.checked_sub_instant(&earlier.0)
    }

    /// Returns the amount of time elapsed from another instant to this one,
    /// or zero duration if that instant is later than this one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use boot_time::{Duration, Instant};
    /// use std::thread::sleep;
    ///
    /// let now = Instant::now();
    /// sleep(Duration::new(1, 0));
    /// let new_now = Instant::now();
    /// println!("{:?}", new_now.saturating_duration_since(now));
    /// println!("{:?}", now.saturating_duration_since(new_now)); // 0ns
    /// ```
    #[must_use]
    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.checked_duration_since(earlier).unwrap_or_default()
    }

    /// Returns the amount of time elapsed since this instant.
    ///
    /// # Panics
    ///
    /// Previous rust versions panicked when the current time was earlier than self. Currently this
    /// method returns a Duration of zero in that case. Future versions may reintroduce the panic.
    /// See [Monotonicity].
    ///
    /// [Monotonicity]: Instant#monotonicity
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::thread::sleep;
    /// use boot_time::{Duration, Instant};
    ///
    /// let instant = Instant::now();
    /// let three_secs = Duration::from_secs(3);
    /// sleep(three_secs);
    /// assert!(instant.elapsed() >= three_secs);
    /// ```
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        Instant::now() - *self
    }

    /// Returns `Some(t)` where `t` is the time `self + duration` if `t` can be represented as
    /// `Instant` (which means it's inside the bounds of the underlying data structure), `None`
    /// otherwise.
    pub fn checked_add(&self, duration: Duration) -> Option<Instant> {
        self.0.checked_add_duration(&duration).map(Instant)
    }

    /// Returns `Some(t)` where `t` is the time `self - duration` if `t` can be represented as
    /// `Instant` (which means it's inside the bounds of the underlying data structure), `None`
    /// otherwise.
    pub fn checked_sub(&self, duration: Duration) -> Option<Instant> {
        self.0.checked_sub_duration(&duration).map(Instant)
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    /// # Panics
    ///
    /// This function may panic if the resulting point in time cannot be represented by the
    /// underlying data structure. See [`Instant::checked_add`] for a version without panic.
    fn add(self, other: Duration) -> Instant {
        self.checked_add(other)
            .expect("overflow when adding duration to instant")
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, other: Duration) {
        *self = *self + other;
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, other: Duration) -> Instant {
        self.checked_sub(other)
            .expect("overflow when subtracting duration from instant")
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, other: Duration) {
        *self = *self - other;
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    /// Returns the amount of time elapsed from another instant to this one,
    /// or zero duration if that instant is later than this one.
    ///
    /// # Panics
    ///
    /// Previous rust versions panicked when `other` was later than `self`. Currently this
    /// method saturates. Future versions may reintroduce the panic in some circumstances.
    /// See [Monotonicity].
    ///
    /// [Monotonicity]: Instant#monotonicity
    fn sub(self, other: Instant) -> Duration {
        self.duration_since(other)
    }
}

impl fmt::Debug for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
