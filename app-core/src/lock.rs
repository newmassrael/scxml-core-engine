// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! One writer at a time per work, across processes.
//!
//! A save reads the current revision, checks that the caller's base is still it,
//! and moves the pointer. Each step is sound alone; the check is worth nothing if
//! another process does its own check and its own move between this one's check
//! and this one's move. So the check and the move are made under an exclusive
//! lock on a file inside the work's folder, and the other process does its check
//! on what this one wrote.
//!
//! The lock is the operating system's (`flock` on Unix, `LockFileEx` on
//! Windows, both behind `std::fs::File::lock`), taken on a file that is never
//! renamed or replaced: a lock on a file that a save swaps in is lost with the
//! swap. It is advisory: it holds off every writer that takes it, which is every
//! writer this crate has, and it does not hold off an editor that does not.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use crate::error::StoreError;

const POLL: Duration = Duration::from_millis(20);

/// A held lock. It is released when this is dropped.
#[derive(Debug)]
pub(crate) struct Held {
    file: File,
}

impl Drop for Held {
    fn drop(&mut self) {
        // Closing the handle releases the lock too; asking first only makes the
        // moment explicit. A failure here has nothing left to protect.
        let _ = self.file.unlock();
    }
}

/// Take the exclusive lock on `path`, waiting up to `wait`.
pub(crate) fn exclusive(path: &Path, wait: Duration) -> Result<Held, StoreError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|source| StoreError::io(path, source))?;
    let started = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(Held { file }),
            Err(TryLockError::WouldBlock) => {
                if started.elapsed() >= wait {
                    return Err(StoreError::Busy {
                        path: path.to_path_buf(),
                        waited_ms: wait.as_millis() as u64,
                    });
                }
                thread::sleep(POLL);
            }
            Err(TryLockError::Error(source)) => return Err(StoreError::io(path, source)),
        }
    }
}
