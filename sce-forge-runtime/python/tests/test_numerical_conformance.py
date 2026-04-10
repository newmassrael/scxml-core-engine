# SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Cross-language numerical conformance harness (Python half).
#
# Fixtures are generated at test-setup time by conftest.py, which calls the
# sce-codegen CLI on the SCXML files in tests/forge/resources/ and inserts
# the output directory into sys.path. This test then imports the generated
# modules and runs them against the reference vectors in
# tests/forge/conformance/numerical_reference.json.
#
# No committed Python goldens are consumed — the single source of truth is
# the SCXML and the codegen. The same reference JSON is used by the Rust,
# C++, Kotlin, and Go conformance tests.
#
# Stdlib-only: json, unittest, pathlib. Runnable under both pytest and
# `python -m unittest`.

import importlib
import json
import unittest
from pathlib import Path

import conftest  # noqa: F401  # triggers bootstrap() at import time

REFERENCE_JSON = (
    conftest.REPO_ROOT / "tests" / "forge" / "conformance" / "numerical_reference.json"
)

filter_moving_average = importlib.import_module("filter_moving_average")
filter_debounce = importlib.import_module("filter_debounce")
interpolation_1d_linear = importlib.import_module("interpolation_1d_linear")
interpolation_2d_bilinear = importlib.import_module("interpolation_2d_bilinear")
observer_coolant = importlib.import_module("observer_coolant")


class TestNumericalConformance(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        with REFERENCE_JSON.open() as f:
            cls.ref = json.load(f)
        cls.tol = cls.ref["float_tolerance"]

    def assert_close(self, actual: float, expected: float, label: str) -> None:
        diff = abs(actual - expected)
        self.assertLessEqual(
            diff,
            self.tol,
            f"{label}: actual={actual}, expected={expected}, diff={diff}, tol={self.tol}",
        )

    def test_interpolation_1d_linear(self) -> None:
        spec = self.ref["pure_functions"]["interpolation_1d_linear"]
        for case in spec["cases"]:
            rpm = case["args"][0]
            expected = case["expected"]
            actual = interpolation_1d_linear.lookup(rpm)
            self.assert_close(actual, expected, f"interpolation_1d_linear({rpm})")

    def test_interpolation_2d_bilinear(self) -> None:
        spec = self.ref["pure_functions"]["interpolation_2d_bilinear"]
        for case in spec["cases"]:
            rpm, load = case["args"]
            expected = case["expected"]
            actual = interpolation_2d_bilinear.lookup(rpm, load)
            self.assert_close(
                actual, expected, f"interpolation_2d_bilinear({rpm}, {load})"
            )

    def test_filter_moving_average(self) -> None:
        spec = self.ref["stateful_filters"]["filter_moving_average"]
        filt = filter_moving_average.FilterMovingAverage()
        for i, step in enumerate(spec["sequence"]):
            actual = filt.update(step["input"])
            self.assert_close(
                actual,
                step["expected"],
                f"filter_moving_average step {i} input={step['input']}",
            )

    def test_filter_debounce(self) -> None:
        spec = self.ref["stateful_filters"]["filter_debounce"]
        filt = filter_debounce.FilterDebounce()
        for i, step in enumerate(spec["sequence"]):
            actual = filt.update(step["input"])
            self.assertEqual(
                actual,
                step["expected"],
                f"filter_debounce step {i} input={step['input']}",
            )

    def test_observer_coolant(self) -> None:
        spec = self.ref["observers"]["observer_coolant"]
        obs = observer_coolant.ObserverCoolant()
        for i, step in enumerate(spec["sequence"]):
            queue = obs.update(step["input"])
            actual = [tag.name for tag in queue]
            self.assertEqual(
                actual,
                step["expected_events"],
                f"observer_coolant step {i} input={step['input']}",
            )


if __name__ == "__main__":
    unittest.main()
