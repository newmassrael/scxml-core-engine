"""What an extractor is, declared as data, and the typed answers a registry gives when none reads a file.

This is the contract alone. The registry that picks an extractor is `extractors.py`, and the extractors
that need a pack (`extractors_pdf.py`) declare themselves here without importing the registry, so that a
registry may list them and they may not need it.
"""

from __future__ import annotations

import pathlib
import pkgutil
from dataclasses import dataclass
from typing import Protocol

from .extraction import Extraction

FORMAT_UNREAD = "format-unread"
PACK_MISSING = "pack-missing"
SNIFF_MISMATCH = "sniff-mismatch"


@dataclass(frozen=True)
class Declaration:
    """What an extractor says of itself."""

    name: str
    version: str
    # Suffixes it reads, lower case with the dot.
    suffixes: tuple[str, ...] = ()
    # Signatures a file of this format begins with. Empty for a format with none (text).
    magic: tuple[bytes, ...] = ()
    # Whether the signature is this format's alone. False for one that other formats share.
    magic_identifies: bool = True
    # The ways of reading it gives (`text-layer`, `ocr`, `vision`).
    ways: tuple[str, ...] = ("text-layer",)
    # The unit a mixed document is chosen by (`page`), or None for a format read whole.
    unit: str | None = None
    # Importable modules it needs, and the name of the pack that carries them.
    needs: tuple[str, ...] = ()
    pack: str | None = None

    def __post_init__(self) -> None:
        if not self.name or not self.version:
            raise ValueError("an extractor declares its name and its version")
        if not self.suffixes and not self.magic:
            raise ValueError(f"extractor {self.name!r} declares neither a suffix nor a signature")
        for suffix in self.suffixes:
            if not suffix.startswith(".") or suffix != suffix.lower():
                raise ValueError(f"extractor {self.name!r} declares the suffix {suffix!r}: lower case, with the dot")
        if self.needs and not self.pack:
            raise ValueError(f"extractor {self.name!r} needs modules and does not name the pack that carries them")

    def available(self) -> bool:
        """Whether what it needs is installed."""
        return not self.missing()

    def missing(self) -> list[str]:
        """The modules it needs that are not installed.

        Looked up by listing what the interpreter could import and never by importing: this module is
        loaded by the server process, which does not import code by name (`test_core_is_domain_free`),
        and a pack that is only looked for must not run its import-time code.
        """
        if not self.needs:
            return []
        installed = {found.name for found in pkgutil.iter_modules()}
        return [module for module in self.needs if module not in installed]

    def has_suffix(self, suffix: str) -> bool:
        return suffix in self.suffixes

    def begins_as_mine(self, head: bytes) -> bool:
        """Whether the first bytes are a signature of this format."""
        return any(head.startswith(sign) for sign in self.magic)

    def is_identified_by_signature(self, head: bytes) -> bool:
        """Whether the first bytes alone say a file is of this format (a PDF's `%PDF-`).

        A signature that other formats share (a zip's, which a word document, a workbook and an
        archive all begin with) checks a file that claims the format and never finds one that does not.
        """
        return self.magic_identifies and self.begins_as_mine(head)


class Extractor(Protocol):
    declaration: Declaration

    def extract(self, path: pathlib.Path) -> Extraction: ...


@dataclass(frozen=True)
class Unreadable:
    """Why a file is not read, as an answer."""

    code: str
    why: str
    pack: str | None = None

    def words(self, path: pathlib.Path) -> str:
        return f"{path}: {self.why}"
