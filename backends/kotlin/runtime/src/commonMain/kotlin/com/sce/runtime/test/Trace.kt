// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The recorded input trace of a test case and its replay
// (docs/SCE_TTCN3_LANE.md §10; docs/SCE_TEST_PROFILE.md §3.2, §3.3, §12.4):
// the inputs of one run from the system under test, each at its logical
// instant, `schemas/sce-test-trace.v1.schema.json`, and the system adapter
// that plays them, [TraceAdapter].
//
// A trace is one text of the JSON Canonicalization Scheme, so that it has one
// spelling and two traces are equal exactly when their bytes are:
// `{"inputs":[{"at","port","type","value"}, …],"schema","suite","test_case"}`.
// `at` is `NOW` in microseconds as the text of its digits, `port` a system
// port of the test case's configuration, and `type` and `value` the record
// type's name and the value in the canonical form a message record of the
// test log writes (§12.5). The inputs stand in order of `at`; the inputs of
// one instant in the order they are appended to their ports' queues (§3.2
// step 2).
//
// The value of an input is read by its type: the reader of the test case's
// configuration ([InputReader], generated as `input` in the configuration's
// unit) knows which records each system port takes and reads each with the
// reader of its record ([FromCanonical]). A trace that does not fit is refused
// whole before the run starts, which then ends with `error` (P13).
//
// Cross-language sibling: `sce_rust_runtime::test::trace` (Rust), which reads
// and refuses the same texts in the same words.

package com.sce.runtime.test

import com.sce.runtime.Json

/** The name of the trace's shape, its `schema`. */
const val TEST_TRACE_SCHEMA: String = "sce-test-trace.v1"

/**
 * The stability status of the trace's wire schema, pinned to the schema file's
 * own `x-sce-schema-status` by a test of the Kotlin runtime and by
 * `schema_file_declares_status` of the Rust one. `SCE_WIRE_CONTRACTS.md`
 * requires one commit to move both.
 */
const val TEST_TRACE_SCHEMA_STATUS: String = "pre-release"

/**
 * Why a trace, or a value of one, is not read: the caller's input, refused by
 * name before anything of the run happens.
 */
class TraceRefusal(
    /** Why. */
    val reason: String,
) : Exception(reason) {
    companion object {
        /**
         * An input of the record type [typeName] on [systemPort], which the
         * configuration whose system ports are [systemPorts] does not take:
         * the port is none of them, or its type's `in` list does not name the
         * type (docs/SCE_TTCN3_LANE.md §10.2).
         */
        fun notAnInput(systemPort: String, typeName: String, systemPorts: List<String>): TraceRefusal =
            if (systemPort in systemPorts) {
                TraceRefusal("'$typeName' is not on the `in` list of the type of the system port '$systemPort'")
            } else {
                TraceRefusal("'$systemPort' is not a system port of the test case's configuration")
            }
    }
}

/**
 * The reader of the inputs of a test case's configuration: the message an
 * input of the record type `typeName` on the system port `systemPort` carries,
 * its value read from the canonical form.
 *
 * Generated as `input` in the unit of a test configuration, which names the
 * records each of its system ports takes.
 *
 * @throws TraceRefusal naming why, where the input is not one of the
 *   configuration's or its value does not belong to its type.
 */
typealias InputReader = (systemPort: String, typeName: String, value: Any?) -> Message

/**
 * The reader of a configuration with no system port: it refuses every input,
 * naming its port.
 *
 * @throws TraceRefusal always.
 */
@Suppress("UNUSED_PARAMETER")
fun noInputs(systemPort: String, typeName: String, value: Any?): Message =
    throw TraceRefusal.notAnInput(systemPort, typeName, emptyList())

/** One input of a trace. */
data class TraceInput(
    /** `NOW` when it is due. */
    val at: Instant,
    /** The system port it arrives on. */
    val port: String,
    /** The name of its record type, as the test log names it. */
    val messageType: String,
    /** Its value, in the canonical form of the test log. */
    val value: Any?,
)

/** A recorded input trace of one test case of one suite. */
data class Trace(
    /** The suite's name. */
    val suite: String,
    /** The test case's name. */
    val testCase: String,
    /**
     * The inputs, in order of `at`, those of one instant in the order they
     * are appended to their ports.
     */
    val inputs: List<TraceInput>,
) {
    /**
     * The trace in its one canonical text, the JSON Canonicalization Scheme,
     * with no line feed after it.
     *
     * @throws Json.NotCanonical when a value holds a number past the range of
     *   a double, which no canonical value holds and [parse] refuses.
     */
    fun canonicalText(): String = Json.writeJcs(
        mapOf(
            "schema" to TEST_TRACE_SCHEMA,
            "suite" to suite,
            "test_case" to testCase,
            "inputs" to inputs.map { input ->
                mapOf(
                    "at" to input.at.toString(),
                    "port" to input.port,
                    "type" to input.messageType,
                    "value" to input.value,
                )
            },
        ),
    )

    companion object {
        /**
         * The trace [text] holds: the JSON Canonicalization Scheme of one
         * trace, which a file may follow with one line feed, as a text file
         * ends.
         *
         * Whether each input fits the suite is the configuration's to read
         * ([TraceAdapter.replaying]).
         *
         * @throws TraceRefusal naming why, where the text is not JSON, the
         *   shape is not `sce-test-trace.v1`, the inputs are out of the order
         *   of their instants, or the text is not the trace's one canonical
         *   spelling.
         */
        fun parse(text: String): Trace {
            val body = text.removeSuffix("\n")
            val value = try {
                Json.parse(body)
            } catch (refused: Json.Error) {
                throw TraceRefusal("the trace is ${refused.message}")
            }
            val (schema, suite, testCase, inputs) =
                members(value, listOf("schema", "suite", "test_case", "inputs"), "the trace")
            if (schema != TEST_TRACE_SCHEMA) {
                throw TraceRefusal("the trace's schema is not $TEST_TRACE_SCHEMA")
            }
            if (inputs !is List<*>) {
                throw TraceRefusal("the trace's inputs are not an array")
            }
            val read = ArrayList<TraceInput>(inputs.size)
            for ((index, item) in inputs.withIndex()) {
                val what = "input $index"
                val (at, port, messageType, held) = members(item, listOf("at", "port", "type", "value"), what)
                val instant = instantOf(at, "the instant of $what")
                val before = read.lastOrNull()
                if (before != null && before.at > instant) {
                    throw TraceRefusal(
                        "$what, at $instant, comes after an input at ${before.at}: the inputs stand in " +
                            "order of their instants",
                    )
                }
                read.add(
                    TraceInput(
                        at = instant,
                        port = nameOf(port, "the port of $what"),
                        messageType = nameOf(messageType, "the type of $what"),
                        value = held,
                    ),
                )
            }
            val trace = Trace(
                suite = nameOf(suite, "the trace's suite"),
                testCase = nameOf(testCase, "the trace's test case"),
                inputs = read,
            )
            // A number past the range of a double has no canonical text, so a
            // trace that holds one is not written in it either.
            val canonical = try {
                trace.canonicalText()
            } catch (_: Json.NotCanonical) {
                null
            }
            if (canonical != body) {
                throw TraceRefusal(
                    "the trace is not written in its one canonical text, the JSON Canonicalization Scheme",
                )
            }
            return trace
        }

        /**
         * The members of [value], an object whose members are [names] exactly,
         * each in the order of [names].
         *
         * @throws TraceRefusal naming [what] otherwise.
         */
        private fun members(value: Any?, names: List<String>, what: String): List<Any?> {
            if (value !is Map<*, *>) {
                throw TraceRefusal("$what is not an object")
            }
            val stray = value.keys.firstOrNull { it !in names }
            if (stray != null) {
                throw TraceRefusal("$what has no member '$stray'")
            }
            return names.map { name ->
                if (!value.containsKey(name)) {
                    throw TraceRefusal("$what has no '$name'")
                }
                value[name]
            }
        }

        /**
         * The text [value], which is not empty, at [what].
         *
         * @throws TraceRefusal otherwise.
         */
        private fun nameOf(value: Any?, what: String): String =
            (value as? String)?.takeIf { it.isNotEmpty() } ?: throw TraceRefusal("$what is not a name")

        /**
         * An instant: the text of the decimal digits of a count of microseconds
         * from the start of the test case, with no leading zero, within `NOW`'s
         * range.
         *
         * @throws TraceRefusal otherwise.
         */
        private fun instantOf(value: Any?, what: String): Instant {
            val digits = (value as? String)
                ?.let { FromCanonical.decimal(it) }
                ?.takeIf { !it.startsWith("-") }
                ?: throw TraceRefusal(
                    "$what is not an instant, the text of the digits of a count of microseconds",
                )
            return digits.toLongOrNull()
                ?: throw TraceRefusal("$what ($digits) is past the microseconds the clock counts to")
        }
    }
}

/** What a component sent to a system port, as a [TraceAdapter] records it. */
data class SystemOutput(
    /** `NOW` when it was sent. */
    val at: Instant,
    /** The system port it was sent to. */
    val systemPort: String,
    /** The message. */
    val message: Message,
)

/**
 * A system adapter that plays a trace (docs/SCE_TTCN3_LANE.md §10.5): at each
 * instant it delivers the inputs due then, in the trace's order, to their
 * system ports (docs/SCE_TEST_PROFILE.md §3.2 step 2), and it records what the
 * components send to the system ports.
 *
 * It announces the instant of its next input ([nextInput]), so when no branch
 * can fire the run's clock advances to the earlier of that instant and the
 * next timer expiry or the guard (§3.3, §12.4 step 4), and the run is the same
 * whenever it is played.
 */
class TraceAdapter private constructor(
    private val inputs: ArrayDeque<Due>,
) : SystemAdapter {
    /** An input read and waiting for its instant. */
    private class Due(val at: Instant, val port: String, val message: Message)

    private val recorded = mutableListOf<SystemOutput>()

    /**
     * What the components have sent to the system ports, in the order they
     * sent it. The caller keeps the adapter, which the run only borrows, so it
     * reads this after the run.
     */
    val sent: List<SystemOutput>
        get() = recorded.toList()

    override fun send(systemPort: String, message: Message, now: Instant) {
        recorded.add(SystemOutput(at = now, systemPort = systemPort, message = message))
    }

    override fun nextInput(): Instant? = inputs.firstOrNull()?.at

    override fun takeInputsDue(now: Instant): List<Pair<String, Message>> {
        val due = mutableListOf<Pair<String, Message>>()
        while (inputs.firstOrNull()?.let { it.at <= now } == true) {
            val input = inputs.removeFirst()
            due.add(input.port to input.message)
        }
        return due
    }

    companion object {
        /**
         * The player of [trace], each input's value read by [read], the reader
         * of the test case's configuration.
         *
         * @throws TraceRefusal naming the input, where one does not fit the
         *   configuration: its port is no system port of it, its type is not
         *   on the `in` list of that port's type, or its value does not belong
         *   to its type (docs/SCE_TTCN3_LANE.md §10.2). The whole trace is
         *   refused.
         */
        fun replaying(trace: Trace, read: InputReader): TraceAdapter {
            val inputs = ArrayDeque<Due>(trace.inputs.size)
            for ((index, input) in trace.inputs.withIndex()) {
                val message = try {
                    read(input.port, input.messageType, input.value)
                } catch (refused: TraceRefusal) {
                    throw TraceRefusal("input $index, at ${input.at}: ${refused.reason}")
                }
                inputs.addLast(Due(input.at, input.port, message))
            }
            return TraceAdapter(inputs)
        }
    }
}
