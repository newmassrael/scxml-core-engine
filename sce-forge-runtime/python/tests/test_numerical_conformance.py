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
transform_temperature = importlib.import_module("transform_temperature")
transform_bitwise = importlib.import_module("transform_bitwise")
transform_multi_output = importlib.import_module("transform_multi_output")
condition_threshold = importlib.import_module("condition_threshold")
condition_range = importlib.import_module("condition_range")
condition_programming = importlib.import_module("condition_programming")
procedure_linear = importlib.import_module("procedure_linear")
procedure_diamond = importlib.import_module("procedure_diamond")
procedure_startup_check = importlib.import_module("procedure_startup_check")


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

    def test_transform_temperature(self) -> None:
        spec = self.ref["pure_functions"]["transform_temperature"]
        for case in spec["cases"]:
            raw = case["args"][0]
            expected = case["expected"]
            actual = transform_temperature.compute_temperature(raw)
            self.assert_close(actual, expected, f"transform_temperature({raw})")

    def test_transform_bitwise(self) -> None:
        spec = self.ref["pure_functions"]["transform_bitwise"]
        for case in spec["cases"]:
            byte = case["args"][0]
            expected = case["expected"]
            actual_high = transform_bitwise.compute_high_nibble(byte)
            actual_low = transform_bitwise.compute_low_nibble(byte)
            self.assertEqual(
                actual_high,
                expected["high_nibble"],
                f"transform_bitwise({byte}) high",
            )
            self.assertEqual(
                actual_low,
                expected["low_nibble"],
                f"transform_bitwise({byte}) low",
            )

    def test_transform_multi_output(self) -> None:
        spec = self.ref["pure_functions"]["transform_multi_output"]
        for case in spec["cases"]:
            celsius = case["args"][0]
            expected = case["expected"]
            actual_f = transform_multi_output.compute_fahrenheit(celsius)
            actual_k = transform_multi_output.compute_kelvin(celsius)
            self.assert_close(
                actual_f,
                expected["fahrenheit"],
                f"transform_multi_output({celsius}) fahrenheit",
            )
            self.assert_close(
                actual_k,
                expected["kelvin"],
                f"transform_multi_output({celsius}) kelvin",
            )

    def test_condition_threshold(self) -> None:
        spec = self.ref["pure_functions"]["condition_threshold"]
        for case in spec["cases"]:
            coolant, oil, maxt = case["args"]
            expected = case["expected"]
            actual = condition_threshold.condition_threshold(coolant, oil, maxt)
            self.assertEqual(
                actual,
                expected,
                f"condition_threshold({coolant}, {oil}, {maxt})",
            )

    def test_condition_range(self) -> None:
        spec = self.ref["pure_functions"]["condition_range"]
        for case in spec["cases"]:
            rpm, min_rpm, max_rpm = case["args"]
            expected = case["expected"]
            actual = condition_range.condition_range(rpm, min_rpm, max_rpm)
            self.assertEqual(
                actual,
                expected,
                f"condition_range({rpm}, {min_rpm}, {max_rpm})",
            )

    def test_condition_programming(self) -> None:
        spec = self.ref["pure_functions"]["condition_programming"]
        for case in spec["cases"]:
            engine_stop, ignition = case["args"]
            expected = case["expected"]
            actual = condition_programming.condition_programming(engine_stop, ignition)
            self.assertEqual(
                actual,
                expected,
                f"condition_programming({engine_stop}, {ignition})",
            )

    def test_procedure_linear(self) -> None:
        spec = self.ref["pure_functions"]["procedure_linear"]
        for case in spec["cases"]:
            value = case["args"][0]
            expected = case["expected"]
            result = procedure_linear.execute(value)
            self.assertEqual(
                result.completed,
                expected["completed"],
                f"procedure_linear({value}).completed",
            )
            self.assertEqual(
                result.final_state,
                expected["final_state"],
                f"procedure_linear({value}).final_state",
            )

    def test_procedure_diamond(self) -> None:
        spec = self.ref["pure_functions"]["procedure_diamond"]
        for case in spec["cases"]:
            sensor, mode = case["args"]
            expected = case["expected"]
            result = procedure_diamond.execute(sensor, mode)
            self.assertEqual(
                result.completed,
                expected["completed"],
                f"procedure_diamond({sensor}, {mode}).completed",
            )
            self.assertEqual(
                result.final_state,
                expected["final_state"],
                f"procedure_diamond({sensor}, {mode}).final_state",
            )

    def test_procedure_startup_check(self) -> None:
        spec = self.ref["pure_functions"]["procedure_startup_check"]
        for case in spec["cases"]:
            voltage, temperature = case["args"]
            expected = case["expected"]
            result = procedure_startup_check.execute(voltage, temperature)
            self.assertEqual(
                result.completed,
                expected["completed"],
                f"procedure_startup_check({voltage}, {temperature}).completed",
            )
            self.assertEqual(
                result.final_state,
                expected["final_state"],
                f"procedure_startup_check({voltage}, {temperature}).final_state",
            )


if __name__ == "__main__":
    unittest.main()
