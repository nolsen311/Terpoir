// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Hybrid logical clock.
//!
//! Stamps follow wall-clock time when clocks are sane, and stay causally
//! ordered when they are not: an edit made after seeing another device's
//! edit always gets a later stamp, even if this device's clock is behind.

use terpoir_model::{DeviceId, Hlc, Stamp};

/// How far ahead of local wall time a remote clock reading may pull this
/// clock. A device with a badly wrong clock cannot drag everyone's stamps
/// into the far future; its own stamps still win merges for the fields it
/// wrote until something newer arrives.
pub const MAX_DRIFT_MS: u64 = 60 * 60 * 1000;

/// A hybrid logical clock for one device. Persist [`Clock::last`] and
/// restore it with [`Clock::new`] so stamps never go backwards across
/// restarts.
#[derive(Clone, Debug)]
pub struct Clock {
    device: DeviceId,
    last: Hlc,
}

impl Clock {
    /// A clock for `device`, resuming after `last` (use `Hlc::default()`
    /// on first run).
    #[must_use]
    pub const fn new(device: DeviceId, last: Hlc) -> Self {
        Self { device, last }
    }

    /// The device this clock stamps for.
    #[must_use]
    pub const fn device(&self) -> DeviceId {
        self.device
    }

    /// The most recent reading, to persist.
    #[must_use]
    pub const fn last(&self) -> Hlc {
        self.last
    }

    /// Stamps a local edit. `now_ms` is the device's wall clock.
    pub fn stamp(&mut self, now_ms: u64) -> Stamp {
        let wall = self.last.wall_ms.max(now_ms);
        self.last = if wall == self.last.wall_ms {
            bump(self.last)
        } else {
            Hlc {
                wall_ms: wall,
                counter: 0,
            }
        };
        Stamp {
            hlc: self.last,
            device: self.device,
        }
    }

    /// Advances the clock past a reading seen in synced data, so later
    /// local edits are stamped after it.
    pub fn observe(&mut self, remote: Hlc, now_ms: u64) {
        let ceiling = now_ms.saturating_add(MAX_DRIFT_MS);
        let remote = if remote.wall_ms > ceiling {
            Hlc {
                wall_ms: ceiling,
                counter: 0,
            }
        } else {
            remote
        };
        if remote > self.last {
            self.last = remote;
        }
    }
}

/// The next reading after `hlc` within the same millisecond, rolling into
/// the next millisecond if the counter is exhausted.
const fn bump(hlc: Hlc) -> Hlc {
    match hlc.counter.checked_add(1) {
        Some(counter) => Hlc {
            wall_ms: hlc.wall_ms,
            counter,
        },
        None => Hlc {
            wall_ms: hlc.wall_ms.saturating_add(1),
            counter: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock() -> Clock {
        Clock::new(DeviceId::from_bytes([1; 16]), Hlc::default())
    }

    #[test]
    fn follows_wall_time() {
        let mut c = clock();
        assert_eq!(
            c.stamp(100).hlc,
            Hlc {
                wall_ms: 100,
                counter: 0
            }
        );
        assert_eq!(
            c.stamp(200).hlc,
            Hlc {
                wall_ms: 200,
                counter: 0
            }
        );
    }

    #[test]
    fn monotonic_when_wall_time_stalls_or_goes_back() {
        let mut c = clock();
        let a = c.stamp(100);
        let b = c.stamp(100);
        let d = c.stamp(50);
        assert!(a < b && b < d);
        assert_eq!(
            d.hlc,
            Hlc {
                wall_ms: 100,
                counter: 2
            }
        );
    }

    #[test]
    fn observe_orders_after_remote_events() {
        let mut c = clock();
        c.stamp(100);
        c.observe(
            Hlc {
                wall_ms: 500,
                counter: 7,
            },
            100,
        );
        let next = c.stamp(100);
        assert_eq!(
            next.hlc,
            Hlc {
                wall_ms: 500,
                counter: 8
            }
        );
    }

    #[test]
    fn observe_caps_far_future_readings() {
        let mut c = clock();
        c.observe(
            Hlc {
                wall_ms: u64::MAX,
                counter: 0,
            },
            1_000,
        );
        assert_eq!(c.last().wall_ms, 1_000 + MAX_DRIFT_MS);
        let next = c.stamp(1_000);
        assert!(next.hlc.wall_ms <= 1_000 + MAX_DRIFT_MS);
    }

    #[test]
    fn counter_overflow_rolls_into_next_millisecond() {
        let mut c = Clock::new(
            DeviceId::from_bytes([1; 16]),
            Hlc {
                wall_ms: 10,
                counter: u32::MAX,
            },
        );
        assert_eq!(
            c.stamp(5).hlc,
            Hlc {
                wall_ms: 11,
                counter: 0
            }
        );
    }

    #[test]
    fn resumes_from_persisted_reading() {
        let mut c = clock();
        c.stamp(1_000);
        let saved = c.last();
        let mut resumed = Clock::new(c.device(), saved);
        assert!(resumed.stamp(0).hlc > saved);
    }
}
