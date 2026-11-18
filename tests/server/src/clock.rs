// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The real clock, as a server reads it.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use sce_rust_mesh::endpoint::Environment;

/// The wall clock, in milliseconds since the Unix epoch: what a delayed send's due is written in,
/// and what the store's index of dues is kept in.
pub fn wall_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// The environment of a server's endpoints: the real clocks, and envelope ids that no other
/// process of the suite gives, so that a server started again after a kill never sends a new
/// envelope under an id its previous life used.
#[derive(Clone)]
pub struct WallClock {
    started: Instant,
}

impl Default for WallClock {
    fn default() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

impl Environment for WallClock {
    fn now_ms(&mut self) -> i64 {
        i64::try_from(self.started.elapsed().as_millis()).unwrap_or(i64::MAX)
    }

    fn now_unix_ms(&mut self) -> u64 {
        wall_ms()
    }

    /// The wall clock, the process's id and a count of the process's own: two processes alive at
    /// once differ in the second, and two lives of one server in the first.
    fn envelope_id(&mut self) -> [u8; 16] {
        static NEXT: AtomicU32 = AtomicU32::new(1);
        let mut id = [0u8; 16];
        id[..8].copy_from_slice(&wall_ms().to_be_bytes());
        id[8..12].copy_from_slice(&std::process::id().to_be_bytes());
        id[12..].copy_from_slice(&NEXT.fetch_add(1, Ordering::Relaxed).to_be_bytes());
        id
    }

    fn jitter_draw(&mut self) -> i64 {
        0
    }
}
