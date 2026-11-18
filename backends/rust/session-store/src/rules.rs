// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The decisions of the contract, written once.
//!
//! Every store reads what a call needs (the lease held on a key, the session's version) inside its
//! transaction, asks these functions whether the call may go on, and writes only if they say so.
//! What differs between stores is where the records live and how a transaction is made; whether a
//! write is fenced out, a lease is held or a renewal comes too late is decided here, so that no
//! two stores can answer the same history differently.

use crate::{moment, Conflict, Held, Lost};

/// The lease a store holds on a key: whose it is, its fence, and when it runs out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Holding {
    pub(crate) owner: String,
    pub(crate) fence: u64,
    pub(crate) expires_ms: u64,
}

/// Whether a put or a delete made under `fence` from version `expected_version` may write, over a
/// key whose lease is `holding` and whose session is at `current_version` (`0` for none). The
/// fence first: a writer that was fenced out is told so, whatever version it read.
pub(crate) fn admit_write(
    holding: Option<&Holding>,
    fence: u64,
    current_version: u64,
    expected_version: u64,
) -> Result<(), Conflict> {
    let current = holding.map(|held| held.fence);
    if current != Some(fence) {
        return Err(Conflict::Fence {
            presented: fence,
            current,
        });
    }
    if current_version != expected_version {
        return Err(Conflict::Version {
            expected: expected_version,
            current: current_version,
        });
    }
    Ok(())
}

/// Whether `owner` may be granted a lease at `now_ms` over a key whose lease is `holding`: unless
/// another owner holds one that has not run out.
pub(crate) fn admit_acquire(
    holding: Option<&Holding>,
    owner: &str,
    now_ms: u64,
) -> Result<(), Held> {
    match holding {
        Some(held) if held.owner != owner && now_ms < held.expires_ms => Err(Held {
            owner: held.owner.clone(),
            expires_ms: held.expires_ms,
        }),
        _ => Ok(()),
    }
}

/// Whether the lease of fence `fence` is the one `holding` records, which a release needs.
pub(crate) fn admit_release(holding: Option<&Holding>, fence: u64) -> Result<(), Lost> {
    match holding {
        Some(held) if held.fence == fence => Ok(()),
        _ => Err(Lost::Superseded {
            current: holding.map(|held| held.fence),
        }),
    }
}

/// Whether the lease of fence `fence` may be renewed at `now_ms`: it is the one held, and it has
/// not run out.
pub(crate) fn admit_renew(holding: Option<&Holding>, fence: u64, now_ms: u64) -> Result<(), Lost> {
    admit_release(holding, fence)?;
    match holding {
        Some(held) if now_ms >= held.expires_ms => Err(Lost::Expired {
            expires_ms: held.expires_ms,
        }),
        _ => Ok(()),
    }
}

/// When a lease granted or renewed at `now_ms` for `ttl_ms` runs out, as a store keeps it.
pub(crate) fn expiry(now_ms: u64, ttl_ms: u64) -> u64 {
    moment(now_ms.saturating_add(ttl_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(owner: &str, fence: u64, expires_ms: u64) -> Holding {
        Holding {
            owner: owner.to_string(),
            fence,
            expires_ms,
        }
    }

    #[test]
    fn a_write_is_fenced_before_its_version_is_compared() {
        let lease = held("a", 7, 100);
        assert_eq!(admit_write(Some(&lease), 7, 3, 3), Ok(()));
        assert_eq!(
            admit_write(Some(&lease), 6, 3, 2),
            Err(Conflict::Fence {
                presented: 6,
                current: Some(7)
            })
        );
        assert_eq!(
            admit_write(None, 7, 0, 0),
            Err(Conflict::Fence {
                presented: 7,
                current: None
            })
        );
        assert_eq!(
            admit_write(Some(&lease), 7, 3, 2),
            Err(Conflict::Version {
                expected: 2,
                current: 3
            })
        );
    }

    #[test]
    fn a_lease_is_held_against_others_until_the_moment_it_runs_out() {
        let lease = held("a", 7, 100);
        assert_eq!(
            admit_acquire(Some(&lease), "b", 99),
            Err(Held {
                owner: "a".to_string(),
                expires_ms: 100
            })
        );
        assert_eq!(admit_acquire(Some(&lease), "b", 100), Ok(()));
        assert_eq!(admit_acquire(Some(&lease), "a", 50), Ok(()));
        assert_eq!(admit_acquire(None, "b", 0), Ok(()));
    }

    #[test]
    fn a_renewal_needs_the_held_lease_before_it_runs_out() {
        let lease = held("a", 7, 100);
        assert_eq!(admit_renew(Some(&lease), 7, 99), Ok(()));
        assert_eq!(
            admit_renew(Some(&lease), 7, 100),
            Err(Lost::Expired { expires_ms: 100 })
        );
        assert_eq!(
            admit_renew(Some(&lease), 6, 50),
            Err(Lost::Superseded { current: Some(7) })
        );
        assert_eq!(
            admit_renew(None, 7, 50),
            Err(Lost::Superseded { current: None })
        );
        assert_eq!(admit_release(Some(&lease), 7), Ok(()));
        assert_eq!(
            admit_release(Some(&lease), 8),
            Err(Lost::Superseded { current: Some(7) })
        );
    }

    #[test]
    fn an_expiry_past_the_latest_moment_is_kept_as_it() {
        assert_eq!(expiry(10, 5), 15);
        assert_eq!(expiry(u64::MAX - 1, 5), crate::LATEST_MOMENT);
        assert_eq!(expiry(crate::LATEST_MOMENT, 1), crate::LATEST_MOMENT);
    }
}
