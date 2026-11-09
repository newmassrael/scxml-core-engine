// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The test log of a test case run (docs/SCE_TEST_PROFILE.md §12.5;
// docs/adr/0091, C3 and answer 8): the records of TCI's test logging interface
// (ETSI ES 201 873-6), each written where the profile's semantics places its
// event, in the order the round scheduler produces them, and the one canonical
// text of a run's log, `schemas/sce-test-log.v1.schema.json`.
//
// The text is the JSON Canonicalization Scheme of the log (`Json.writeJcs`), and
// it holds no number: an instant, a duration and every integer of a value are
// texts of their digits ([Canonical]). So two runs agree, under C3, exactly when
// their logs are the same bytes, whichever runtime wrote each.
//
// Cross-language sibling: `sce_rust_runtime::test::TestLogRecord` and
// `TestCaseResult::canonical_log` (Rust), whose text this one writes byte for
// byte.

package com.sce.runtime.test

import com.sce.runtime.Json

/** The name of the test log's shape, its `schema`. */
const val TEST_LOG_SCHEMA: String = "sce-test-log.v1"

/** One record of the test log: an event and `NOW` when it happened. */
data class TestLogRecord(
    /** `NOW` when it was recorded. */
    val at: Instant,
    /** What happened. */
    val event: TestLogEvent,
) {
    /** The record as the test log writes it: `at`, `kind`, and the members of its kind. */
    fun toValue(): Map<String, Any> {
        val members = LinkedHashMap<String, Any>()
        members["at"] = Canonical.integer(at)
        members["kind"] = event.kind
        event.writeMembers(members)
        return members
    }
}

/**
 * An event of TCI's test logging interface (ES 201 873-6), as the profile's
 * semantics places it (docs/SCE_TEST_PROFILE.md §12.5).
 *
 * A component is named `mtc` or by the PTC's name in its configuration.
 */
sealed interface TestLogEvent {
    /** The record's `kind` in the test log: the TCI event in kebab case. */
    val kind: String

    /** Writes the members of its kind, beside `at` and `kind`, into [members]. */
    fun writeMembers(members: MutableMap<String, Any>)

    /** The test case started, at `NOW = 0` (P11). */
    data object TestCaseStarted : TestLogEvent {
        override val kind: String
            get() = "test-case-started"

        override fun writeMembers(members: MutableMap<String, Any>) {}
    }

    /** The test case ended with its [verdict] (P14). */
    data class TestCaseEnded(val verdict: Verdict) : TestLogEvent {
        override val kind: String
            get() = "test-case-ended"

        override fun writeMembers(members: MutableMap<String, Any>) {
            members["verdict"] = verdict.spelling
        }
    }

    /**
     * The [component] of the type [componentType] was created: the MTC as the
     * test case starts, a PTC by the invocation that creates it (P9).
     */
    data class ComponentCreated(val component: String, val componentType: String) : TestLogEvent {
        override val kind: String
            get() = "component-created"

        override fun writeMembers(members: MutableMap<String, Any>) {
            members["component"] = component
            members["type"] = componentType
        }
    }

    /** The [component] terminated, with its local [verdict] then (P10). */
    data class ComponentDone(val component: String, val verdict: Verdict) : TestLogEvent {
        override val kind: String
            get() = "component-done"

        override fun writeMembers(members: MutableMap<String, Any>) {
            members["component"] = component
            members["verdict"] = verdict.spelling
        }
    }

    /**
     * The [component] sent a message on [port] (P6): its record type's name
     * ([TestValue.typeName]) and its [value] in canonical form.
     */
    data class MessageSent(
        val component: String,
        val port: String,
        val messageType: String,
        val value: Any,
    ) : TestLogEvent {
        override val kind: String
            get() = "message-sent"

        override fun writeMembers(members: MutableMap<String, Any>) {
            writeMessage(members, component, port, messageType, value)
        }
    }

    /**
     * A receive branch of the [component] took the head of [port] (P4 step
     * 1): the message was received and matched.
     */
    data class MessageReceived(
        val component: String,
        val port: String,
        val messageType: String,
        val value: Any,
    ) : TestLogEvent {
        override val kind: String
            get() = "message-received"

        override fun writeMembers(members: MutableMap<String, Any>) {
            writeMessage(members, component, port, messageType, value)
        }
    }

    /** The [component] started [timer] for [duration] microseconds (P7). */
    data class TimerStarted(val component: String, val timer: String, val duration: Duration) : TestLogEvent {
        override val kind: String
            get() = "timer-started"

        override fun writeMembers(members: MutableMap<String, Any>) {
            members["component"] = component
            members["timer"] = timer
            members["duration_us"] = Canonical.integer(duration)
        }
    }

    /**
     * The [component] stopped [timer], which was running (P7). A stop of a
     * timer that is not running changes nothing, and is not recorded.
     */
    data class TimerStopped(val component: String, val timer: String) : TestLogEvent {
        override val kind: String
            get() = "timer-stopped"

        override fun writeMembers(members: MutableMap<String, Any>) {
            members["component"] = component
            members["timer"] = timer
        }
    }

    /** A timeout branch of the [component] took [timer], which had timed out (P4 step 1). */
    data class TimerTimedOut(val component: String, val timer: String) : TestLogEvent {
        override val kind: String
            get() = "timer-timed-out"

        override fun writeMembers(members: MutableMap<String, Any>) {
            members["component"] = component
            members["timer"] = timer
        }
    }

    /**
     * The local verdict of the [component] was set: by `<sce:setverdict>`
     * (P12), or to `error` by the runtime (P13). The [reason] is the
     * document's, and none is recorded for an empty one or for the runtime's
     * `error`, whose cause is the runtime's own prose, which another runtime
     * would not write alike.
     */
    data class VerdictSet(val component: String, val verdict: Verdict, val reason: String?) : TestLogEvent {
        override val kind: String
            get() = "verdict-set"

        override fun writeMembers(members: MutableMap<String, Any>) {
            members["component"] = component
            members["verdict"] = verdict.spelling
            if (reason != null) {
                members["reason"] = reason
            }
        }
    }

    /** A `<log>` of the [component] (P8). */
    data class Log(val component: String, val text: String) : TestLogEvent {
        override val kind: String
            get() = "log"

        override fun writeMembers(members: MutableMap<String, Any>) {
            members["component"] = component
            members["text"] = text
        }
    }
}

/** The members a message sent and a message received both write. */
private fun writeMessage(
    members: MutableMap<String, Any>,
    component: String,
    port: String,
    messageType: String,
    value: Any,
) {
    members["component"] = component
    members["port"] = port
    members["type"] = messageType
    members["value"] = value
}

/**
 * The test log of the run of [testCase] of the suite [suite], which ended with
 * [verdict] and wrote [records], in its one canonical text: the JSON
 * Canonicalization Scheme of `schema`, `suite`, `test_case`, `verdict` and
 * `records`, each record in the order the run produced it.
 *
 * @throws IllegalStateException when a message's value is not in the canonical
 *   form ([TestValue.canonical]) but holds a number past the range of a double,
 *   or a value of a type that is no JSON value, which no writer of [Canonical]
 *   builds.
 */
internal fun writeTestLog(
    suite: String,
    testCase: String,
    verdict: Verdict,
    records: List<TestLogRecord>,
): String {
    val log = mapOf(
        "schema" to TEST_LOG_SCHEMA,
        "suite" to suite,
        "test_case" to testCase,
        "verdict" to verdict.spelling,
        "records" to records.map { it.toValue() },
    )
    return try {
        Json.writeJcs(log)
    } catch (refused: Json.NotCanonical) {
        throw IllegalStateException("the test log of $suite $testCase is not canonical: ${refused.reason}", refused)
    }
}
