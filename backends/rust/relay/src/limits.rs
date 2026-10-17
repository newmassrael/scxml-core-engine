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
        let (held, at) = self
            .spent
            .entry((list.to_string(), node))
            .or_insert((capacity, now_ms));
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
}
