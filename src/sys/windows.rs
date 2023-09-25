use std::time::Duration;

use core::hash::Hash;

trait IsZero {
    fn is_zero(&self) -> bool;
}

macro_rules! impl_is_zero {
    ($($t:ident)*) => ($(impl IsZero for $t {
        fn is_zero(&self) -> bool {
            *self == 0
        }
    })*)
}

impl_is_zero! { i8 i16 i32 i64 isize u8 u16 u32 u64 usize }

fn cvt<I: IsZero>(i: I) -> std::io::Result<I> {
    if i.is_zero() {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(i)
    }
}

const NANOS_PER_SEC: u64 = 1_000_000_000;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct Instant {
    // This duration is relative to an arbitrary microsecond epoch
    // from the winapi QueryPerformanceCounter function.
    t: Duration,
}

impl Instant {
    pub fn now() -> Instant {
        // High precision timing on windows operates in "Performance Counter"
        // units, as returned by the WINAPI QueryPerformanceCounter function.
        // These relate to seconds by a factor of QueryPerformanceFrequency.
        // In order to keep unit conversions out of normal interval math, we
        // measure in QPC units and immediately convert to nanoseconds.
        perf_counter::PerformanceCounterInstant::now().into()
    }

    pub fn checked_sub_instant(&self, other: &Instant) -> Option<Duration> {
        // On windows there's a threshold below which we consider two timestamps
        // equivalent due to measurement error. For more details + doc link,
        // check the docs on epsilon.
        let epsilon = perf_counter::PerformanceCounterInstant::epsilon();
        if other.t > self.t && other.t - self.t <= epsilon {
            Some(Duration::new(0, 0))
        } else {
            self.t.checked_sub(other.t)
        }
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant {
            t: self.t.checked_add(*other)?,
        })
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant {
            t: self.t.checked_sub(*other)?,
        })
    }
}

mod perf_counter {
    use super::cvt;
    use super::NANOS_PER_SEC;
    use crate::sys_common::mul_div_u64;
    use std::os::raw::c_longlong;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    type BOOL = i32;

    #[link(name = "kernel32")]
    extern "system" {
        pub fn QueryPerformanceCounter(lpperformancecount: *mut i64) -> BOOL;
    }
    #[link(name = "kernel32")]
    extern "system" {
        pub fn QueryPerformanceFrequency(lpfrequency: *mut i64) -> BOOL;
    }

    #[allow(non_camel_case_types)]
    pub type LARGE_INTEGER = c_longlong;

    pub struct PerformanceCounterInstant {
        ts: LARGE_INTEGER,
    }
    impl PerformanceCounterInstant {
        pub fn now() -> Self {
            Self { ts: query() }
        }

        // Per microsoft docs, the margin of error for cross-thread time comparisons
        // using QueryPerformanceCounter is 1 "tick" -- defined as 1/frequency().
        // Reference: https://docs.microsoft.com/en-us/windows/desktop/SysInfo
        //                   /acquiring-high-resolution-time-stamps
        pub fn epsilon() -> Duration {
            let epsilon = NANOS_PER_SEC / (frequency() as u64);
            Duration::from_nanos(epsilon)
        }
    }
    impl From<PerformanceCounterInstant> for super::Instant {
        fn from(other: PerformanceCounterInstant) -> Self {
            let freq = frequency() as u64;
            let instant_nsec = mul_div_u64(other.ts as u64, NANOS_PER_SEC, freq);
            Self {
                t: Duration::from_nanos(instant_nsec),
            }
        }
    }

    fn frequency() -> LARGE_INTEGER {
        // Either the cached result of `QueryPerformanceFrequency` or `0` for
        // uninitialized. Storing this as a single `AtomicU64` allows us to use
        // `Relaxed` operations, as we are only interested in the effects on a
        // single memory location.
        static FREQUENCY: AtomicU64 = AtomicU64::new(0);

        let cached = FREQUENCY.load(Ordering::Relaxed);
        // If a previous thread has filled in this global state, use that.
        if cached != 0 {
            return cached as LARGE_INTEGER;
        }
        // ... otherwise learn for ourselves ...
        let mut frequency = 0;
        unsafe {
            cvt(QueryPerformanceFrequency(&mut frequency)).unwrap();
        }

        FREQUENCY.store(frequency as u64, Ordering::Relaxed);
        frequency
    }

    fn query() -> LARGE_INTEGER {
        let mut qpc_value: LARGE_INTEGER = 0;
        cvt(unsafe { QueryPerformanceCounter(&mut qpc_value) }).unwrap();
        qpc_value
    }
}
