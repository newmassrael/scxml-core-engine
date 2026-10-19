// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Reading a socket directly before sleeping on the reactor (SCE
//! Protocol-Synthesis RFC §synth-5-Q, Session runtime of a tokio host).
//!
//! `machines.<m>.scheduler.session_runtime.rx_busy_poll_us` is how long a read
//! that has to wait reads the socket with `read(2)` before it gives up the
//! thread. This module is the reading: the host passes it the budget the
//! generated `SESSION_RUNTIME` carries, so the declared figure is consumed by
//! code SCE owns and tests, and a host does not write the loop again.
//!
//! # Why `read(2)` and not the reactor's `try_read`
//!
//! A tokio `try_read` answers from the readiness the reactor last recorded, so
//! a spin on it returns at once until the reactor has run, and the budget only
//! delays the sleep (measured by the host that asked for this: a 30 us budget
//! cost 38 us, a 100 us one 108 us). Reading the descriptor itself asks the
//! kernel, which is what makes a wait that ends inside the budget cheaper than
//! the sleep and the wake-up.
//!
//! # What it costs
//!
//! A read that waits spends the whole budget of CPU, and a wait that outlasts
//! it has spent the budget for nothing before it sleeps. That is why the value
//! is a deploy declaration a node chooses and not a default, and why a budget
//! longer than the scheduler's own tick is refused at deploy time
//! (`deploy/scheduler-rx-busy-poll-invalid`).

use core::time::Duration;
use std::fs::File;
use std::io::{self, Read};
use std::mem::ManuallyDrop;
use std::os::fd::{AsRawFd, FromRawFd};
use std::time::Instant;

/// Read from `socket` into `buf`, trying again for at most `budget`.
///
/// `socket` must be non-blocking: a blocking descriptor sleeps in the kernel
/// and the budget never starts. Answers
///
/// - `Ok(Some(n))` when the descriptor became readable inside the budget; `n`
///   is the count read, and 0 is the peer's end of stream, as `read(2)` says;
/// - `Ok(None)` when the budget ran out with nothing to read, which is the
///   caller's cue to wait for readiness on its reactor;
/// - `Err(e)` for any error but `WouldBlock`, which is how the budget is spent,
///   and `Interrupted`, which is retried.
///
/// A zero `budget` makes one attempt, so a host with the feature off pays one
/// system call and not a loop.
pub fn read_within<S: AsRawFd>(
    socket: &S,
    buf: &mut [u8],
    budget: Duration,
) -> io::Result<Option<usize>> {
    // The descriptor is the caller's: wrapping it in a `File` to reach `read(2)`
    // must not close it, so the wrapper is never dropped.
    //
    // SAFETY: `socket` is borrowed for the whole call, so the descriptor is open
    // and stays open until we return; the wrapper does not outlive the call and
    // is never dropped, so the descriptor is not closed twice.
    let mut file = ManuallyDrop::new(unsafe { File::from_raw_fd(socket.as_raw_fd()) });
    let deadline = Instant::now() + budget;
    loop {
        match file.read(buf) {
            Ok(read) => return Ok(Some(read)),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Ok(None);
                }
                core::hint::spin_loop();
            }
            Err(error) => return Err(error),
        }
    }
}
