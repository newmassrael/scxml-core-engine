"""What the server needs of the Python it runs in, written down once.

The server is the authority on what it needs, and other places have to agree with it: the question
an application asks before it starts the server (`--check`), the packages an installer declares
(the workbench's `.deb`), and the packages a CI machine is given to try the installer with. They
agree because each reads this list or is held to it (`tests/test_the_installer_declares_what_the_
server_needs.py`, `scripts/verify_installed_app.sh`), and not because somebody copied it: a list
written in four places is four lists, and the one that is out of date reads like the others.
"""

from __future__ import annotations

import importlib
from dataclasses import dataclass


@dataclass(frozen=True)
class Need:
    module: str         # what `import` takes
    name: str           # what the owner reads
    pip: str            # what `pip install` takes
    debian: str         # the package that provides it on Debian and Ubuntu
    cost: str | None    # what stops without it, when that is not everything


NEEDS = (
    Need("yaml", "PyYAML", "pyyaml", "python3-yaml", None),
    Need("jsonschema", "jsonschema", "jsonschema", "python3-jsonschema",
         "without it the owner's answers and a pack cannot be read"),
)


def _can_import(module: str) -> bool:
    """Whether `module` imports here, which is what the server will ask of it."""
    try:
        importlib.import_module(module)
    except ImportError:
        return False
    return True


def missing() -> list[Need]:
    """The needs this Python does not meet, in the order they are listed."""
    return [need for need in NEEDS if not _can_import(need.module)]


def sentence(need: Need) -> str:
    """One sentence for an owner: what is missing and how to get it, and what stops without it."""
    said = f"{need.name} is not installed for this Python: pip install {need.pip}"
    return said if need.cost is None else f"{said} ({need.cost})"
