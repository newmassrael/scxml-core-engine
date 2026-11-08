# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The module parameters of a test run (docs/SCE_TEST_PROFILE.md §11.5, §11.8;
ETSI ES 201 873-1 §8.2): the value a suite gives each, which every component
that declares a module parameter of that id reads into its variable before its
machine starts.

A value is held as the one JSON value model a saved state is made of (a
``JsonNumber``, a ``str``, a ``bool``, a ``list``, a ``dict`` or ``None``). The
generated suite writes each default through the ``saved_write_*`` function of
its declared type (``sce_runtime.saved_values``), and the generated component
reads it back through the ``saved_read_*`` function of the type it declares,
which are the reads a restored variable is held by: a value of another type, or
past its bound, is refused by name rather than converted. The runtime knows the
type of no module parameter, so it is written once for every suite.

Cross-language siblings: ``sce_rust_runtime::test::ModuleParameters`` (Rust),
``SCE::Test::ModuleParameters`` (C++),
``com.sce.runtime.test.ModuleParameters`` (Kotlin),
``testprofile.ModuleParameters`` (Go).
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional

from ..saved_state import StateRefusal

#: A module parameter's value: the saved form of a value of its declared type.
ModuleParameterValue = Any


class ModuleParameters:
    """The module parameters a run hands its components, by id.

    Generated from a ``test-suite`` with each parameter at its default, and given
    other values by the caller of a run where it overrides one (-1 §8.2.1). A
    new set holds no parameter.
    """

    __slots__ = ("_by_id",)

    def __init__(self) -> None:
        self._by_id: Dict[str, ModuleParameterValue] = {}

    def set(self, id: str, value: ModuleParameterValue) -> None:  # noqa: A002 -- the profile's word
        """Give the module parameter ``id`` the value ``value``, in place of any
        it had."""
        self._by_id[id] = value

    def has(self, id: str) -> bool:  # noqa: A002 -- the profile's word
        """Whether the module parameter ``id`` has a value; ``None`` is a
        value."""
        return id in self._by_id

    def of(self, id: str) -> ModuleParameterValue:  # noqa: A002 -- the profile's word
        """The value of the module parameter ``id``.

        Raises a ``StateRefusal`` naming it when it has none, since no value is
        made up in its place.
        """
        if id not in self._by_id:
            raise StateRefusal(f"no value is given for the module parameter '{id}'")
        return self._by_id[id]

    def ids(self) -> List[str]:
        """The ids of every module parameter that has a value, in ascending
        order."""
        return sorted(self._by_id)

    def copy(self) -> "ModuleParameters":
        """Another set of module parameters with the same values, which a change
        to either leaves apart."""
        copied = ModuleParameters()
        copied._by_id = dict(self._by_id)
        return copied

    # A set of module parameters is mutable, so it compares by its values and,
    # defining `__eq__` and no `__hash__`, is no key.
    def __eq__(self, other: object) -> bool:
        return isinstance(other, ModuleParameters) and other._by_id == self._by_id

    def __repr__(self) -> str:
        by_id = {key: self._by_id[key] for key in self.ids()}
        return f"ModuleParameters({by_id!r})"


def copy_of(parameters: Optional[ModuleParameters]) -> ModuleParameters:
    """A copy of ``parameters``, or an empty set for ``None``, which every caller
    of a run may give for no module parameter."""
    return parameters.copy() if parameters is not None else ModuleParameters()
