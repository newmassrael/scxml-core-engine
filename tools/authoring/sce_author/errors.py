"""The one thing a caller has to catch.

Every way this tool can decline -- a pack it cannot use, a document it cannot
read, a file that is not there -- is one exception type with one property: the
message names the path and says what was wrong with it. A caller that catches
`AuthoringError` has caught all of them, and a caller that catches nothing gets
an exit code rather than a traceback.

⚠ This exists because it was measured missing. Eleven ordinary mistakes were
put to the command line -- a path that is not there, a directory where a file
belongs, a symlink pointing at itself, bytes that are not text, a YAML file
that is binary, an output path that cannot be written -- and SEVEN produced a
traceback. A traceback tells the caller where this program is, not what they
did wrong, and for half of them it did not even name the file.

The rule that follows: anything reading something the caller named catches the
library exception it can raise and re-raises it as one of these. `except
Exception` is not that rule -- each site names the errors that reading can
actually produce, so a bug inside this tool still surfaces as a bug.
"""

from __future__ import annotations

import pathlib


class AuthoringError(Exception):
    """A refusal a caller can act on. Names the path and what was wrong."""


class PackError(AuthoringError):
    """A pack could not be used. Always names the file and what was wrong."""


class IngestError(AuthoringError):
    """A document could not be read. Always names what would read it."""


# What reading a file off a disk can raise, other than the file simply not
# being there: a directory in its place, a permission, a symlink loop, a
# device that is not a file, bytes that are not text in the encoding claimed.
READ_ERRORS = (OSError, UnicodeDecodeError, ValueError)


def describe_path(path: pathlib.Path) -> str:
    """Why this path cannot be read, in the caller's terms rather than errno.

    `is_file()` answers false for a directory, for a missing file and for a
    symlink that points at itself, and reporting all three as "no such file"
    sent a reader looking for a file that was sitting right there.
    """
    path = pathlib.Path(path)
    if path.is_dir():
        return f"{path}: is a directory, and a document was expected"
    if path.is_symlink():
        return f"{path}: is a symlink that does not lead to a file"
    if not path.exists():
        return f"{path}: no such file"
    return f"{path}: is not a regular file"
