// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the tests of the core share: a transport that records what it was handed and fails as
//! told, and an environment whose clock the test moves.

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::endpoint::{Environment, Transport, TransportFailure};

/// A transport that records what it sent and fails as told.
#[derive(Default)]
pub(crate) struct Recorder {
    pub sent: Vec<(String, Vec<u8>)>,
    pub failures: VecDeque<TransportFailure>,
}

impl Transport for Recorder {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        if let Some(failure) = self.failures.pop_front() {
            return Err(failure);
        }
        self.sent.push((peer.to_string(), bytes.to_vec()));
        Ok(())
    }
}

/// A clock the test moves, ids that count, and no jitter.
#[derive(Default)]
pub(crate) struct Fixed {
    pub now: i64,
    pub ids: u8,
    /// Which life of a process this is: the ids of one are told apart from
    /// another's, as UUID v7s drawn by two processes are. The first life, `0`,
    /// draws `[n; 16]` for its `n`th id.
    pub incarnation: u8,
}

impl Environment for Fixed {
    fn now_ms(&mut self) -> i64 {
        self.now
    }
    /// The wall clock a fixed distance ahead of the monotonic one, so a
    /// wire deadline and a kept one can be told apart.
    fn now_unix_ms(&mut self) -> u64 {
        1_000_000 + self.now as u64
    }
    fn envelope_id(&mut self) -> [u8; 16] {
        self.ids += 1;
        let mut id = [self.ids; 16];
        if self.incarnation > 0 {
            id[0] = 0xF0 | self.incarnation;
        }
        id
    }
    fn jitter_draw(&mut self) -> i64 {
        0
    }
}
