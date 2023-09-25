use super::{Duration, Instant};
use core::fmt::Debug;

const SECOND: Duration = Duration::from_secs(1);

macro_rules! assert_almost_eq {
    ($a:expr, $b:expr) => {{
        let (a, b) = ($a, $b);
        if a != b {
            let (a, b) = if a > b { (a, b) } else { (b, a) };
            assert!(
                a - Duration::from_micros(1) <= b,
                "{:?} is not almost equal to {:?}",
                a,
                b
            );
        }
    }};
}

#[test]
fn instant_monotonic() {
    let a = Instant::now();
    loop {
        let b = Instant::now();
        assert!(b >= a);
        if b > a {
            break;
        }
    }
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn instant_monotonic_concurrent() -> std::thread::Result<()> {
    let threads: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(|| {
                let mut old = Instant::now();
                let count = if cfg!(miri) { 1_000 } else { 5_000_000 };
                for _ in 0..count {
                    let new = Instant::now();
                    assert!(new >= old);
                    old = new;
                }
            })
        })
        .collect();
    for t in threads {
        t.join()?;
    }
    Ok(())
}

#[test]
fn instant_elapsed() {
    let a = Instant::now();
    let _ = a.elapsed();
}

#[test]
fn instant_math() {
    let a = Instant::now();
    let b = Instant::now();
    println!("a: {a:?}");
    println!("b: {b:?}");
    let dur = b.duration_since(a);
    println!("dur: {dur:?}");
    assert_almost_eq!(b - dur, a);
    assert_almost_eq!(a + dur, b);

    let second = SECOND;
    assert_almost_eq!(a - second + second, a);
    assert_almost_eq!(
        a.checked_sub(second).unwrap().checked_add(second).unwrap(),
        a
    );

    // checked_add_duration will not panic on overflow
    let mut maybe_t = Some(Instant::now());
    let max_duration = Duration::from_secs(u64::MAX);
    // in case `Instant` can store `>= now + max_duration`.
    for _ in 0..2 {
        maybe_t = maybe_t.and_then(|t| t.checked_add(max_duration));
    }
    assert_eq!(maybe_t, None);

    // checked_add_duration calculates the right time and will work for another year
    let year = Duration::from_secs(60 * 60 * 24 * 365);
    assert_eq!(a + year, a.checked_add(year).unwrap());
}

#[test]
fn instant_math_is_associative() {
    let now = Instant::now();
    let offset = Duration::from_millis(5);
    // Changing the order of instant math shouldn't change the results,
    // especially when the expression reduces to X + identity.
    assert_eq!((now + offset) - now, (now - now) + offset);

    // On any platform, `Instant` should have the same resolution as `Duration` (e.g. 1 nanosecond)
    // or better. Otherwise, math will be non-associative (see #91417).
    let now = Instant::now();
    let provided_offset = Duration::from_nanos(1);
    let later = now + provided_offset;
    let measured_offset = later - now;
    assert_eq!(measured_offset, provided_offset);
}

#[test]
fn instant_duration_since_saturates() {
    let a = Instant::now();
    assert_eq!((a - SECOND).duration_since(a), Duration::ZERO);
}

#[test]
fn instant_checked_duration_since_nopanic() {
    let now = Instant::now();
    let earlier = now - SECOND;
    let later = now + SECOND;
    assert_eq!(earlier.checked_duration_since(now), None);
    assert_eq!(later.checked_duration_since(now), Some(SECOND));
    assert_eq!(now.checked_duration_since(now), Some(Duration::ZERO));
}

#[test]
fn instant_saturating_duration_since_nopanic() {
    let a = Instant::now();
    #[allow(deprecated, deprecated_in_future)]
    let ret = (a - SECOND).saturating_duration_since(a);
    assert_eq!(ret, Duration::ZERO);
}

#[test]
fn big_math() {
    // Check that the same result occurs when adding/subtracting each duration one at a time as when
    // adding/subtracting them all at once.
    #[track_caller]
    fn check<T: Eq + Copy + Debug>(start: Option<T>, op: impl Fn(&T, Duration) -> Option<T>) {
        const DURATIONS: [Duration; 2] =
            [Duration::from_secs(i64::MAX as _), Duration::from_secs(50)];
        if let Some(start) = start {
            assert_eq!(
                op(&start, DURATIONS.into_iter().sum()),
                DURATIONS.into_iter().try_fold(start, |t, d| op(&t, d))
            )
        }
    }

    let instant = Instant::now();
    check(
        instant.checked_sub(Duration::from_secs(100)),
        Instant::checked_add,
    );
    check(
        instant.checked_sub(Duration::from_secs(i64::MAX as _)),
        Instant::checked_add,
    );
    check(
        instant.checked_add(Duration::from_secs(100)),
        Instant::checked_sub,
    );
    check(
        instant.checked_add(Duration::from_secs(i64::MAX as _)),
        Instant::checked_sub,
    );
}
