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
// The generated Forge Kotlin fixtures do not declare a package, so this
// test file deliberately stays in the default (unnamed) package — classes
// in the unnamed package cannot be imported from named packages on the JVM.

import com.sce.forge.runtime.EventQueue
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

    @Test
    fun interpolation1dLinearMatchesReference() {
        val spec = ref["pure_functions"]!!.jsonObject["interpolation_1d_linear"]!!.jsonObject
        for (case in spec["cases"]!!.jsonArray) {
            val rpm = case.jsonObject["args"]!!.jsonArray[0].jsonPrimitive.int.toUShort()
            val expected = case.jsonObject["expected"]!!.jsonPrimitive.double
            val actual = Interpolation1dLinear.lookup(rpm)
            assertClose(actual, expected, "interpolation_1d_linear($rpm)")
        }
    }

    @Test
    fun interpolation2dBilinearMatchesReference() {
        val spec = ref["pure_functions"]!!.jsonObject["interpolation_2d_bilinear"]!!.jsonObject
        for (case in spec["cases"]!!.jsonArray) {
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
}
