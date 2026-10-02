"""One writer at a time, across processes, for the files this package replaces.

A file that two people keep together (an authoring profile every drafter under it
reads) is saved by reading it, adding to what was read, checking that nobody
changed it meanwhile, and renaming the result over it. Each step is sound alone,
and the check is worth nothing when another process does its own check and its own
rename between this one's check and this one's rename: both pass, both are told
they saved, and the file keeps only the later. Measured 2026-10-02 with two real
processes held at that point.

So the check and the rename are made together, under a lock the other process
cannot take until this one is done, and the other process then does its check on
what this one wrote.

The lock is taken on the DIRECTORY the file lives in, not on the file and not on a
lock file beside it. A lock on the file is lost with the rename (the name then
points at another inode), and a lock file is one more thing in the owner's folder
that a crash can leave behind. A directory outlives every save, and holding it
costs nothing but that two saves into one folder wait for each other.

⚠ It is advisory: it holds off every writer that takes it, which is every save
this package makes, and it does not hold off an editor that does not. That
editor is what the digest check is for, and the window it leaves is the one the
check always had.

⚠ POSIX only. Where `fcntl` is missing the lock is REFUSED, not skipped: a save
that cannot be made safe says so and writes nothing.
"""

from __future__ import annotations

import contextlib
import os
import pathlib
import time
from collections.abc import Iterator

#: How long a save waits for another to finish before it gives up and says so.
DEFAULT_WAIT_SECONDS = 30.0

_POLL_SECONDS = 0.02


class LockUnavailable(RuntimeError):
    """The lock could not be taken, so nothing is to be written under it."""


@contextlib.contextmanager
def exclusive(directory: pathlib.Path, wait_seconds: float = DEFAULT_WAIT_SECONDS) -> Iterator[None]:
    """Hold the lock on `directory` for the body, waiting up to `wait_seconds`."""
    try:
        import fcntl
    except ImportError as error:
        raise LockUnavailable(
            "this platform offers no inter-process file lock here, and a save that cannot "
            "be made safe against another save is not made") from error
    try:
        descriptor = os.open(directory, os.O_RDONLY)
    except OSError as error:
        raise LockUnavailable(f"{directory} cannot be opened to take a lock ({error})") from error
    try:
        deadline = time.monotonic() + wait_seconds
        while True:
            try:
                fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    raise LockUnavailable(
                        f"another save into {directory} did not finish within "
                        f"{wait_seconds:g} s") from None
                time.sleep(_POLL_SECONDS)
            except OSError as error:
                raise LockUnavailable(f"{directory} cannot be locked ({error})") from error
        try:
            yield
        finally:
            fcntl.flock(descriptor, fcntl.LOCK_UN)
    finally:
        os.close(descriptor)
