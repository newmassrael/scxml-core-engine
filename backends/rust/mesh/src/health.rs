// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a server's parts say of themselves to the program's readiness and liveness checks
//! (docs/adr/0109, D16): `HttpsServer::health` and [`crate::pool::Pool::health`].
//!
//! A part is in one of three states: serving (it takes new work in), stopping (a stop has begun
//! and what it holds is still being answered or saved), and stopped (it takes nothing in and
//! holds nothing). Beside its state it says whether the store it keeps its state in answered a
//! read just now. A program answers its readiness check with [`Health::ready`], so that a load
//! balancer sends nothing to a process that stops or cannot reach its store, and its liveness
//! check with whether `health` answered at all, which a process stuck past help does not: a
//! store that does not answer makes a process unready, not dead, since restarting it reaches the
//! same store.

/// What a part of a server says of itself. See the module documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    /// It takes new work in: false from the moment a stop begins.
    pub accepting: bool,
    /// A stop has begun and has not ended: what it held is still being answered or saved.
    pub stopping: bool,
    /// The store it keeps its state in answered a read just now; true for a part that keeps
    /// none.
    pub store_reachable: bool,
}

impl Health {
    /// Whether the part should be sent work: it takes work in and its store answers.
    pub fn ready(&self) -> bool {
        self.accepting && self.store_reachable
    }

    /// Whether the part has stopped: it takes nothing in and its stop has ended, so the program
    /// may exit.
    pub fn stopped(&self) -> bool {
        !self.accepting && !self.stopping
    }

    /// The health of a program made of `self` and `other`: it accepts only while both do, is
    /// stopping while either is, and reaches its stores only while both do.
    pub fn and(self, other: Health) -> Health {
        Health {
            accepting: self.accepting && other.accepting,
            stopping: self.stopping || other.stopping,
            store_reachable: self.store_reachable && other.store_reachable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A part is ready only while it accepts and reaches its store, stopped only once it neither
    /// accepts nor stops, and a program is as healthy as its least healthy part.
    #[test]
    fn readiness_needs_both_accepting_and_a_store_that_answers() {
        let serving = Health {
            accepting: true,
            stopping: false,
            store_reachable: true,
        };
        let unreachable = Health {
            store_reachable: false,
            ..serving
        };
        let stopping = Health {
            accepting: false,
            stopping: true,
            store_reachable: true,
        };
        let stopped = Health {
            stopping: false,
            ..stopping
        };
        assert!(serving.ready() && !serving.stopped());
        assert!(!unreachable.ready() && !unreachable.stopped());
        assert!(!stopping.ready() && !stopping.stopped());
        assert!(!stopped.ready() && stopped.stopped());
        assert_eq!(serving.and(unreachable), unreachable);
        assert_eq!(serving.and(stopping), stopping);
        assert_eq!(stopped.and(stopping), stopping);
    }
}
