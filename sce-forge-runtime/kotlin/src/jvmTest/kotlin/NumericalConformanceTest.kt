// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Cross-language numerical conformance harness (Kotlin/JVM half).
//
// The fixtures referenced below are generated at configure time by the
// `generateForgeFixtures` Gradle task, which invokes sce-codegen on the
// SCXML files in tests/forge/resources/ and writes the Kotlin output into
// build/generated/conformance/kotlin/. That directory is wired into the
// jvmTest source set, so the generated classes land in this test binary's
// classpath alongside this file. No committed Kotlin goldens are consumed;
// the single source of truth is the SCXML and the codegen.
//
// All Kotlin Forge templates emit `package com.sce.generated.<name>`, so
// this test imports each fixture from its named package. The test file
// itself is package-free only because keeping it in the default package
// avoids having to pick an arbitrary package name for a single test class.

import com.sce.forge.runtime.EventQueue
import com.sce.generated.condition_programming.conditionProgramming
import com.sce.generated.condition_range.conditionRange
import com.sce.generated.condition_threshold.conditionThreshold
import com.sce.generated.filter_debounce.FilterDebounce
import com.sce.generated.filter_moving_average.FilterMovingAverage
import com.sce.generated.interpolation_1d_linear.Interpolation1dLinear
import com.sce.generated.interpolation_2d_bilinear.Interpolation2dBilinear
import com.sce.generated.observer_coolant.ForgeDomainTag
import com.sce.generated.observer_coolant.ObserverCoolant
import com.sce.generated.procedure_diamond.ProcedureDiamond
import com.sce.generated.procedure_diamond.ProcedureResult as DiamondResult
import com.sce.generated.procedure_linear.ProcedureLinear
import com.sce.generated.procedure_linear.ProcedureResult as LinearResult
import com.sce.generated.procedure_startup_check.ProcedureResult as StartupResult
import com.sce.generated.procedure_startup_check.ProcedureStartupCheck
import com.sce.generated.transform_bitwise.computeHighNibble
import com.sce.generated.transform_bitwise.computeLowNibble
import com.sce.generated.transform_multi_output.computeFahrenheit
import com.sce.generated.transform_multi_output.computeKelvin
import com.sce.generated.transform_temperature.computeTemperature
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.double
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.nio.file.Paths
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class NumericalConformanceTest {
    private val ref: JsonObject
    private val tol: Double

    init {
        val repoRoot = System.getProperty("sce.repo.root")
            ?: error("sce.repo.root system property not set by Gradle")
        val refPath = Paths.get(repoRoot, "tests", "forge", "conformance", "numerical_reference.json")
        ref = Json.parseToJsonElement(refPath.toFile().readText()).jsonObject
        tol = ref["float_tolerance"]!!.jsonPrimitive.double
    }

    private fun assertClose(actual: Double, expected: Double, label: String) {
        val diff = abs(actual - expected)
        assertTrue(
            diff <= tol,
            "$label: actual=$actual expected=$expected diff=$diff tol=$tol",
        )
    }

    private fun pureCases(name: String) =
        ref["pure_functions"]!!.jsonObject[name]!!.jsonObject["cases"]!!.jsonArray

    @Test
    fun interpolation1dLinearMatchesReference() {
        for (case in pureCases("interpolation_1d_linear")) {
            val rpm = case.jsonObject["args"]!!.jsonArray[0].jsonPrimitive.int.toUShort()
            val expected = case.jsonObject["expected"]!!.jsonPrimitive.double
            val actual = Interpolation1dLinear.lookup(rpm)
            assertClose(actual, expected, "interpolation_1d_linear($rpm)")
        }
    }

    @Test
    fun interpolation2dBilinearMatchesReference() {
        for (case in pureCases("interpolation_2d_bilinear")) {
            val args = case.jsonObject["args"]!!.jsonArray
            val rpm = args[0].jsonPrimitive.int.toUShort()
            val load = args[1].jsonPrimitive.int.toUByte()
            val expected = case.jsonObject["expected"]!!.jsonPrimitive.double
            val actual = Interpolation2dBilinear.lookup(rpm, load)
            assertClose(actual, expected, "interpolation_2d_bilinear($rpm, $load)")
        }
    }

    @Test
    fun filterMovingAverageMatchesReference() {
        val spec = ref["stateful_filters"]!!.jsonObject["filter_moving_average"]!!.jsonObject
        val filter = FilterMovingAverage()
        for ((i, step) in spec["sequence"]!!.jsonArray.withIndex()) {
            val input = step.jsonObject["input"]!!.jsonPrimitive.double
            val expected = step.jsonObject["expected"]!!.jsonPrimitive.double
            val actual = filter.update(input)
            assertClose(actual, expected, "filter_moving_average step $i input=$input")
        }
    }

    @Test
    fun filterDebounceMatchesReference() {
        val spec = ref["stateful_filters"]!!.jsonObject["filter_debounce"]!!.jsonObject
        val filter = FilterDebounce()
        for ((i, step) in spec["sequence"]!!.jsonArray.withIndex()) {
            val input = step.jsonObject["input"]!!.jsonPrimitive.boolean
            val expected = step.jsonObject["expected"]!!.jsonPrimitive.boolean
            val actual = filter.update(input)
            assertEquals(expected, actual, "filter_debounce step $i input=$input")
        }
    }

    @Test
    fun observerCoolantMatchesReference() {
        val spec = ref["observers"]!!.jsonObject["observer_coolant"]!!.jsonObject
        val observer = ObserverCoolant()
        for ((i, step) in spec["sequence"]!!.jsonArray.withIndex()) {
            val input = step.jsonObject["input"]!!.jsonPrimitive.double
            val expectedEvents = step.jsonObject["expected_events"]!!.jsonArray.map {
                it.jsonPrimitive.content
            }
            val queue: EventQueue<ForgeDomainTag> = observer.update(input)
            val actualEvents = queue.asList().map { it.name }
            assertEquals(expectedEvents, actualEvents, "observer_coolant step $i input=$input")
        }
    }

    @Test
    fun transformTemperatureMatchesReference() {
        for (case in pureCases("transform_temperature")) {
            val raw = case.jsonObject["args"]!!.jsonArray[0].jsonPrimitive.int.toUShort()
            val expected = case.jsonObject["expected"]!!.jsonPrimitive.double
            val actual = computeTemperature(raw)
            assertClose(actual, expected, "transform_temperature($raw)")
        }
    }

    @Test
    fun transformBitwiseMatchesReference() {
        for (case in pureCases("transform_bitwise")) {
            val byte = case.jsonObject["args"]!!.jsonArray[0].jsonPrimitive.int.toUByte()
            val expected = case.jsonObject["expected"]!!.jsonObject
            val expectedHigh = expected["high_nibble"]!!.jsonPrimitive.int.toUByte()
            val expectedLow = expected["low_nibble"]!!.jsonPrimitive.int.toUByte()
            val actualHigh = computeHighNibble(byte)
            val actualLow = computeLowNibble(byte)
            assertEquals(expectedHigh, actualHigh, "transform_bitwise($byte) high")
            assertEquals(expectedLow, actualLow, "transform_bitwise($byte) low")
        }
    }

    @Test
    fun transformMultiOutputMatchesReference() {
        for (case in pureCases("transform_multi_output")) {
            val celsius = case.jsonObject["args"]!!.jsonArray[0].jsonPrimitive.double
            val expected = case.jsonObject["expected"]!!.jsonObject
            val expectedF = expected["fahrenheit"]!!.jsonPrimitive.double
            val expectedK = expected["kelvin"]!!.jsonPrimitive.double
            val actualF = computeFahrenheit(celsius)
            val actualK = computeKelvin(celsius)
            assertClose(actualF, expectedF, "transform_multi_output($celsius) fahrenheit")
            assertClose(actualK, expectedK, "transform_multi_output($celsius) kelvin")
        }
    }

    @Test
    fun conditionThresholdMatchesReference() {
        for (case in pureCases("condition_threshold")) {
            val args = case.jsonObject["args"]!!.jsonArray
            val coolant = args[0].jsonPrimitive.double
            val oil = args[1].jsonPrimitive.double
            val maxTemp = args[2].jsonPrimitive.double
            val expected = case.jsonObject["expected"]!!.jsonPrimitive.boolean
            val actual = conditionThreshold(coolant, oil, maxTemp)
            assertEquals(expected, actual, "condition_threshold($coolant, $oil, $maxTemp)")
        }
    }

    @Test
    fun conditionRangeMatchesReference() {
        for (case in pureCases("condition_range")) {
            val args = case.jsonObject["args"]!!.jsonArray
            val rpm = args[0].jsonPrimitive.int.toUInt()
            val minRpm = args[1].jsonPrimitive.int.toUInt()
            val maxRpm = args[2].jsonPrimitive.int.toUInt()
            val expected = case.jsonObject["expected"]!!.jsonPrimitive.boolean
            val actual = conditionRange(rpm, minRpm, maxRpm)
            assertEquals(expected, actual, "condition_range($rpm, $minRpm, $maxRpm)")
        }
    }

    @Test
    fun conditionProgrammingMatchesReference() {
        for (case in pureCases("condition_programming")) {
            val args = case.jsonObject["args"]!!.jsonArray
            val engineStop = args[0].jsonPrimitive.boolean
            val ignition = args[1].jsonPrimitive.boolean
            val expected = case.jsonObject["expected"]!!.jsonPrimitive.boolean
            val actual = conditionProgramming(engineStop, ignition)
            assertEquals(expected, actual, "condition_programming($engineStop, $ignition)")
        }
    }

    @Test
    fun procedureLinearMatchesReference() {
        for (case in pureCases("procedure_linear")) {
            val value = case.jsonObject["args"]!!.jsonArray[0].jsonPrimitive.int
            val expected = case.jsonObject["expected"]!!.jsonObject
            val expectedCompleted = expected["completed"]!!.jsonPrimitive.boolean
            val expectedFinal = expected["final_state"]!!.jsonPrimitive.content
            val result: LinearResult = ProcedureLinear.execute(value)
            assertEquals(expectedCompleted, result.completed, "procedure_linear($value).completed")
            assertEquals(expectedFinal, result.finalState, "procedure_linear($value).final_state")
        }
    }

    @Test
    fun procedureDiamondMatchesReference() {
        for (case in pureCases("procedure_diamond")) {
            val args = case.jsonObject["args"]!!.jsonArray
            val sensor = args[0].jsonPrimitive.int.toUShort()
            val mode = args[1].jsonPrimitive.content
            val expected = case.jsonObject["expected"]!!.jsonObject
            val expectedCompleted = expected["completed"]!!.jsonPrimitive.boolean
            val expectedFinal = expected["final_state"]!!.jsonPrimitive.content
            val result: DiamondResult = ProcedureDiamond.execute(sensor, mode)
            assertEquals(expectedCompleted, result.completed, "procedure_diamond($sensor, $mode).completed")
            assertEquals(expectedFinal, result.finalState, "procedure_diamond($sensor, $mode).final_state")
        }
    }

    @Test
    fun procedureStartupCheckMatchesReference() {
        for (case in pureCases("procedure_startup_check")) {
            val args = case.jsonObject["args"]!!.jsonArray
            val voltage = args[0].jsonPrimitive.double.toFloat()
            val temperature = args[1].jsonPrimitive.double.toFloat()
            val expected = case.jsonObject["expected"]!!.jsonObject
            val expectedCompleted = expected["completed"]!!.jsonPrimitive.boolean
            val expectedFinal = expected["final_state"]!!.jsonPrimitive.content
            val result: StartupResult = ProcedureStartupCheck.execute(voltage, temperature)
            assertEquals(expectedCompleted, result.completed, "procedure_startup_check($voltage, $temperature).completed")
            assertEquals(expectedFinal, result.finalState, "procedure_startup_check($voltage, $temperature).final_state")
        }
    }
}
