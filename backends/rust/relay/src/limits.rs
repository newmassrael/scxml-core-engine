// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a relay holds a device to.
//!
//! A relay reads nothing, so it cannot tell a useful write from a useless one; what it can do is
//! bound how much any one device asks of it. Each device of each list has an allowance that refills
//! with time (a token bucket in whole milli-units, so no float and no clock but the one the caller
//! passes), a write costs more than a read, and the size of what a request carries and of what a
//! reply returns is bounded.

use std::collections::BTreeMap;

/// The bounds a relay applies. The defaults suit a household list; an operator sets their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// What a device may spend at once, in requests.
    pub burst: u32,
    /// What a device's allowance gains each second, in requests.
    pub refill_per_second: u32,
    /// What a write costs, in requests: a read costs one.
    pub write_cost: u32,
    /// The longest request text the relay takes, in bytes.
    pub max_request_bytes: usize,
    /// The most records a feed returns at once, whatever the request asks.
    pub max_feed: u32,
    /// The most operations a range returns at once.
    pub max_range: u64,
    /// The most records the relay keeps for one list; past it, writes are refused until the
    /// operator has made room.
    pub max_records_per_list: usize,
    /// The most lists the relay holds; past it, the first entry of another is refused. A relay of
    /// several slots (`Shards`) holds this many in each. Lowering it refuses new lists and keeps the
    /// ones held: what a relay holds bounds its memory as this times `max_records_per_list` times
    /// the largest record.
    pub max_lists: usize,
    /// The largest MLS message (a key package, a commit, a Welcome) the relay keeps, in bytes: a
    /// Welcome carries the group's tree, so a large group needs room.
    pub max_group_message_bytes: usize,
    /// How many key packages one device may have left with the relay at a time.
    pub max_key_packages_per_node: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            burst: 120,
            refill_per_second: 20,
            write_cost: 2,
            max_request_bytes: 64 * 1024,
            max_feed: 500,
            max_range: 500,
            max_records_per_list: 1_000_000,
            max_lists: 10_000,
            max_group_message_bytes: 32 * 1024,
            max_key_packages_per_node: 16,
        }
    }
}

/// The allowance of each device of each list.
#[derive(Default)]
pub(crate) struct Allowances {
    /// Milli-requests left and the time (ms) they were counted at, by list and node.
    spent: BTreeMap<(String, u64), (u64, u64)>,
}

impl Allowances {
    /// Spend `cost` requests of the device's allowance at `now_ms`. `Err` is how long, in
    /// milliseconds, until it has that much again.
    pub(crate) fn take(
        &mut self,
        list: &str,
        node: u64,
        cost: u32,
        now_ms: u64,
        limits: &Limits,
    ) -> Result<(), u64> {
        let capacity = u64::from(limits.burst) * 1000;
        let wanted = u64::from(cost.max(1)).min(u64::from(limits.burst).max(1)) * 1000;
        let refill = u64::from(limits.refill_per_second);
        let bucket = self
            .spent
            .entry((list.to_string(), node))
            .or_insert((capacity, now_ms));
        spend(bucket, capacity, wanted, refill, now_ms)
    }

    /// Forget the allowance of every device of `list`: a list that lapsed, whose devices start
    /// again as new ones when it is created again.
    pub(crate) fn forget(&mut self, list: &str) {
        self.spent.retain(|(held, _), _| held != list);
    }
}

/// Spend `wanted` units of the token bucket `bucket` (what it holds, and the time in ms it was
/// counted at) at `now_ms`, after it gained `refill` units for each millisecond since, up to
/// `capacity`. `Err` is how long, in milliseconds, until it holds `wanted` again. Every allowance of
/// a relay is one of these: a device's, and each table of [`Windows`].
fn spend(
    bucket: &mut (u64, u64),
    capacity: u64,
    wanted: u64,
    refill: u64,
    now_ms: u64,
) -> Result<(), u64> {
    let (held, at) = bucket;
    // A clock that goes back adds nothing, and never takes away.
    let gained = now_ms.saturating_sub(*at).saturating_mul(refill);
    *held = held.saturating_add(gained).min(capacity);
    *at = (*at).max(now_ms);
    if *held >= wanted {
        *held -= wanted;
        Ok(())
    } else if refill == 0 {
        Err(u64::MAX)
    } else {
        Err((wanted - *held).div_ceil(refill))
    }
}

/// So many of a thing for each key in each window of time, as a token bucket for each key that a
/// window fills again: what a relay keeps per source and per node for those that log in (ADR 0080
/// section 5). A key whose bucket is full is the same as a key never seen, so a key is forgotten once
/// a window has passed since it last spent, and the table holds at most `most` keys: past it, a key
/// it does not hold waits for the next sweep, so that a flood of keys costs a bounded table.
pub(crate) struct Windows<K> {
    spent: BTreeMap<K, (u64, u64)>,
    swept_at: u64,
    most: usize,
}

impl<K: Ord> Windows<K> {
    pub(crate) fn new(most: usize) -> Self {
        Self {
            spent: BTreeMap::new(),
            swept_at: 0,
            most,
        }
    }

    /// Spend one of the `per_window` of `key` in each `window_ms` at `now_ms`. `Err` is how long, in
    /// milliseconds, until it may. A request is `window_ms` units, and a bucket gains `per_window` of
    /// them each millisecond, so that it is full again a window after it was spent.
    pub(crate) fn take(
        &mut self,
        key: K,
        per_window: u32,
        window_ms: u64,
        now_ms: u64,
    ) -> Result<(), u64> {
        let (window_ms, refill, capacity) = bucket_of(per_window, window_ms);
        self.room_for(&key, window_ms, now_ms)?;
        let bucket = self.spent.entry(key).or_insert((capacity, now_ms));
        spend(bucket, capacity, window_ms, refill, now_ms)
    }

    /// What [`Self::take`] would answer for `key` at `now_ms`, with nothing spent: so that a caller
    /// refuses for the rate before it does anything, and spends only once what it judged holds.
    /// Only the sweep of the keys a window has passed since they spent is done, which changes no
    /// answer: a key whose bucket is full is the same as a key never seen.
    pub(crate) fn admits(
        &mut self,
        key: &K,
        per_window: u32,
        window_ms: u64,
        now_ms: u64,
    ) -> Result<(), u64> {
        let (window_ms, refill, capacity) = bucket_of(per_window, window_ms);
        self.room_for(key, window_ms, now_ms)?;
        let mut bucket = self.spent.get(key).copied().unwrap_or((capacity, now_ms));
        spend(&mut bucket, capacity, window_ms, refill, now_ms)
    }

    /// Sweep the table when a window has passed since it was last swept, and say whether it has
    /// room for `key`: it holds it, or holds fewer keys than its bound. `Err` is how long until the
    /// next sweep.
    fn room_for(&mut self, key: &K, window_ms: u64, now_ms: u64) -> Result<(), u64> {
        if now_ms.saturating_sub(self.swept_at) >= window_ms {
            self.spent
                .retain(|_, (_, at)| now_ms.saturating_sub(*at) < window_ms);
            self.swept_at = now_ms;
        }
        if !self.spent.contains_key(key) && self.spent.len() >= self.most {
            return Err(window_ms.saturating_sub(now_ms.saturating_sub(self.swept_at)));
        }
        Ok(())
    }

    /// How many keys the table holds.
    pub(crate) fn len(&self) -> usize {
        self.spent.len()
    }

    /// Whether the table holds `key`: after a refusal, one it does not hold was refused because the
    /// table holds as many keys as it may, and not for its own rate.
    pub(crate) fn holds(&self, key: &K) -> bool {
        self.spent.contains_key(key)
    }
}

/// The window (at least a millisecond), the units a bucket gains each millisecond (at least one) and
/// what it holds full, for `per_window` in each `window_ms`.
fn bucket_of(per_window: u32, window_ms: u64) -> (u64, u64, u64) {
    let window_ms = window_ms.max(1);
    let refill = u64::from(per_window.max(1));
    (window_ms, refill, refill.saturating_mul(window_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> Limits {
        Limits {
            burst: 4,
            refill_per_second: 2,
            write_cost: 2,
            ..Limits::default()
        }
    }

    #[test]
    fn a_device_spends_its_burst_and_then_waits_for_the_refill() {
        let mut allowances = Allowances::default();
        let limits = limits();
        for _ in 0..4 {
            assert_eq!(allowances.take("home", 1, 1, 0, &limits), Ok(()));
        }
        // Two requests a second: half a second for the next.
        assert_eq!(allowances.take("home", 1, 1, 0, &limits), Err(500));
        assert_eq!(allowances.take("home", 1, 1, 499, &limits), Err(1));
        assert_eq!(allowances.take("home", 1, 1, 500, &limits), Ok(()));
        // A write costs two.
        assert_eq!(allowances.take("home", 1, 2, 500, &limits), Err(1000));
        assert_eq!(allowances.take("home", 1, 2, 1500, &limits), Ok(()));
    }

    #[test]
    fn one_device_spending_its_allowance_leaves_another_theirs() {
        let mut allowances = Allowances::default();
        let limits = limits();
        for _ in 0..4 {
            allowances.take("home", 1, 1, 0, &limits).unwrap();
        }
        assert!(allowances.take("home", 1, 1, 0, &limits).is_err());
        assert!(allowances.take("home", 2, 1, 0, &limits).is_ok());
        assert!(
            allowances.take("work", 1, 1, 0, &limits).is_ok(),
            "another list"
        );
    }

    #[test]
    fn the_devices_of_a_list_forgotten_start_with_their_burst_and_others_keep_theirs() {
        let mut allowances = Allowances::default();
        let limits = limits();
        for _ in 0..4 {
            allowances.take("home", 1, 1, 0, &limits).unwrap();
            allowances.take("work", 1, 1, 0, &limits).unwrap();
        }
        allowances.forget("home");
        assert!(allowances.take("home", 1, 1, 0, &limits).is_ok());
        assert!(
            allowances.take("work", 1, 1, 0, &limits).is_err(),
            "another list's allowance is kept"
        );
    }

    #[test]
    fn the_allowance_never_passes_the_burst_and_a_clock_that_goes_back_adds_nothing() {
        let mut allowances = Allowances::default();
        let limits = limits();
        allowances.take("home", 1, 1, 1_000_000, &limits).unwrap();
        for _ in 0..3 {
            allowances.take("home", 1, 1, 1_000_000, &limits).unwrap();
        }
        assert!(allowances.take("home", 1, 1, 1_000_000, &limits).is_err());
        assert!(allowances.take("home", 1, 1, 0, &limits).is_err());
    }

    #[test]
    fn a_device_that_was_away_for_long_has_its_burst_and_not_more() {
        let mut allowances = Allowances::default();
        let limits = limits();
        // It spends one of its four, and is away for a day: what it has is the four, not a day's worth.
        allowances.take("home", 1, 1, 0, &limits).unwrap();
        for _ in 0..4 {
            allowances.take("home", 1, 1, 86_400_000, &limits).unwrap();
        }
        assert!(
            allowances.take("home", 1, 1, 86_400_000, &limits).is_err(),
            "the burst is the most it holds"
        );
    }

    #[test]
    fn a_relay_that_refills_nothing_says_so_and_does_not_divide_by_zero() {
        let mut allowances = Allowances::default();
        let limits = Limits {
            burst: 1,
            refill_per_second: 0,
            ..Limits::default()
        };
        assert!(allowances.take("home", 1, 1, 0, &limits).is_ok());
        assert_eq!(allowances.take("home", 1, 1, 10, &limits), Err(u64::MAX));
    }

    #[test]
    fn a_window_gives_so_many_to_each_key_and_fills_again_as_it_passes() {
        let mut windows = Windows::new(16);
        for _ in 0..3 {
            assert_eq!(windows.take("a", 3, 60_000, 0), Ok(()));
        }
        // Three a minute: the next one a third of a minute after the first.
        assert_eq!(windows.take("a", 3, 60_000, 0), Err(20_000));
        assert_eq!(windows.take("a", 3, 60_000, 19_999), Err(1));
        assert_eq!(windows.take("a", 3, 60_000, 20_000), Ok(()));
        assert_eq!(windows.take("b", 3, 60_000, 20_000), Ok(()), "another key");
        // A window after it last spent, a key is as full as one never seen, and is forgotten.
        assert_eq!(windows.len(), 2);
        assert_eq!(windows.take("c", 3, 60_000, 80_000), Ok(()));
        assert_eq!(windows.len(), 1, "a and b were forgotten");
        for _ in 0..2 {
            assert_eq!(windows.take("c", 3, 60_000, 80_000), Ok(()));
        }
        assert!(windows.take("c", 3, 60_000, 80_000).is_err());
    }

    /// What `admits` answers is what `take` then answers, at every instant of a window and for a
    /// key held, a key not held and a key a full table refuses, and `admits` spends nothing: the
    /// oracle is `take` on a copy of the table.
    #[test]
    fn a_window_admits_what_it_would_take_and_spends_nothing() {
        let fresh = || {
            let mut windows = Windows::new(2);
            windows.take("a", 3, 60_000, 0).unwrap();
            windows.take("a", 3, 60_000, 0).unwrap();
            windows.take("b", 3, 60_000, 0).unwrap();
            windows
        };
        for now in [0, 1, 19_999, 20_000, 40_000, 59_999, 60_000, 120_000] {
            for key in ["a", "b", "c"] {
                let mut asked = fresh();
                let mut spent = fresh();
                let admitted = asked.admits(&key, 3, 60_000, now);
                assert_eq!(admitted, spent.take(key, 3, 60_000, now), "{key} at {now}");
                // Asked again, the same answer: nothing was spent.
                assert_eq!(
                    asked.admits(&key, 3, 60_000, now),
                    admitted,
                    "{key} at {now}"
                );
                assert!(!asked.holds(&"c"), "admits holds no key it did not hold");
            }
        }
        let mut windows = fresh();
        assert_eq!(
            windows.admits(&"c", 3, 60_000, 0),
            Err(60_000),
            "the table is full"
        );
        assert!(!windows.holds(&"c"), "a refused key is not held");
        for _ in 0..5 {
            assert!(windows.admits(&"b", 3, 60_000, 0).is_ok());
        }
        assert!(windows.take("b", 3, 60_000, 0).is_ok());
        assert!(windows.take("b", 3, 60_000, 0).is_ok());
        assert!(
            windows.take("b", 3, 60_000, 0).is_err(),
            "admits spent none of the three"
        );
    }

    #[test]
    fn a_window_holds_at_most_so_many_keys_and_a_new_one_waits_for_the_sweep() {
        let mut windows = Windows::new(2);
        assert!(windows.take(1, 5, 1_000, 0).is_ok());
        assert!(windows.take(2, 5, 1_000, 0).is_ok());
        assert_eq!(
            windows.take(3, 5, 1_000, 400),
            Err(600),
            "the table is full"
        );
        assert!(windows.take(1, 5, 1_000, 400).is_ok(), "a key it holds");
        assert!(
            windows.take(3, 5, 1_000, 1_400).is_ok(),
            "the sweep made room"
        );
    }
}
