// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Installing an executable stand-in that a test is about to run.
//!
//! Every test that fakes a tool writes a small script and runs it — directly,
//! or through a process it spawns (cmake, a gate script). Written with
//! `fs::write`, that races every other test thread of the same binary. A
//! thread that forks while the write is open hands its child the write
//! descriptor, and until that child execs — its `O_CLOEXEC` copy closes then —
//! exec of the stand-in fails with `ETXTBSY` ("Text file busy"). Measured
//! 2026-09-25 as `formatter::tests::only_cpp_sources_reach_clang_format`
//! failing once and passing on the rerun, the clang-format stand-in reported
//! as "does not run: Text file busy (os error 26)".
//!
//! Renaming a written file into place does not close the window: the
//! inherited descriptor names the inode, and a rename keeps it. Writing the
//! stand-ins once before the threads start is not available either — the test
//! harness starts them. What closes it is this process never opening the file
//! for writing at all, so a child writes it: `sh` copies its stdin into place
//! and marks it executable, and the write descriptor lives and dies in that
//! child.
//!
//! One source for both halves of the crate's tests: the integration tests
//! reach it as `common::executable`, and the library's unit tests include this
//! file by path.

#![allow(dead_code)]

use std::io;
use std::path::Path;

/// Put `contents` at `path` with the executable bit set, without this process
/// ever holding `path` open for writing. The parent directory must exist.
#[cfg(unix)]
pub fn install_executable(path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("sh")
        .arg("-c")
        .arg("cat > \"$1\" && chmod 755 \"$1\"")
        .arg("sh")
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()?;
    // Written and dropped in one statement, so `cat` sees end of input.
    child
        .stdin
        .take()
        .expect("the child's stdin was piped")
        .write_all(contents.as_ref())?;
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "installing the executable {} failed: {status}",
            path.display()
        )))
    }
}

/// Off unix there is no executable bit to set and no fork to race.
#[cfg(not(unix))]
pub fn install_executable(path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    std::fs::write(path, contents)
}
