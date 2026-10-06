# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A string variable of a ``sce-static`` machine is held to the UTF-8 bytes it
declares, which is what every backend counts: two characters of four bytes fit a
bound of four and not of three, and two of five bytes fit neither, so a count of
characters is told apart from a count of bytes. A byte string is held to the
bytes it declares by the same helper, counted as they are."""

import pytest

from sce_forge_runtime import algorithm


@pytest.mark.parametrize(
    "value, capacity",
    [("abcd", 4), ("éé", 4), ("", 1)],
)
def test_a_value_that_fits_comes_back_unchanged(value, capacity):
    assert algorithm.bounded(value, capacity) == value


@pytest.mark.parametrize(
    "value, capacity",
    [("abcde", 4), ("éé", 3), ("é€", 4)],
)
def test_a_value_past_its_bytes_is_a_capacity_failure(value, capacity):
    with pytest.raises(algorithm.AlgorithmFailure) as failure:
        algorithm.bounded(value, capacity)
    assert failure.value.error is algorithm.AlgorithmError.CAPACITY_EXCEEDED


@pytest.mark.parametrize(
    "value, capacity",
    [(b"abcd", 4), (b"\xe9\xff\x80\x00", 4), (b"", 1)],
)
def test_a_byte_string_that_fits_comes_back_unchanged(value, capacity):
    assert algorithm.bounded(value, capacity) == value


@pytest.mark.parametrize(
    "value, capacity",
    [(b"abcde", 4), ("é".encode("utf-8"), 1)],
)
def test_a_byte_string_past_its_bytes_is_a_capacity_failure(value, capacity):
    with pytest.raises(algorithm.AlgorithmFailure) as failure:
        algorithm.bounded(value, capacity)
    assert failure.value.error is algorithm.AlgorithmError.CAPACITY_EXCEEDED
