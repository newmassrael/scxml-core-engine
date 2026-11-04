// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0057 §7 — the configuration a saved state was taken in. A machine of a
// product line is built for one configuration, the features its build turns on,
// and its saved state records them as `features`, sorted, each once, as the last
// member of every session. A restore judges them before the shape and refuses a
// state of another configuration by the features that differ; a machine of no
// product line leaves the member out, so its text is the one it was before.
//
// The Rust twins are the tests `a_saved_state_is_read_back_as_it_was_written`,
// `a_state_of_another_configuration_is_refused_by_the_features_that_differ` and
// `the_features_of_a_saved_state_are_a_sorted_set` of
// `backends/rust/runtime/src/saved_state.rs`.

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

private object ConfiguredState : State

private sealed interface ConfiguredEvent : Event {
    data object Ping : ConfiguredEvent
}

/**
 * The smallest machine that saves and restores, bound to [shape]: what a
 * generated machine's `save` and `restore` call, reached the way generated code
 * reaches them.
 */
private class ConfiguredMachine(private val shape: SavedShape) :
    StateMachineEngine<ConfiguredState, ConfiguredEvent>() {
    override val initialState: ConfiguredState = ConfiguredState

    // A saved state names its session (docs/adr/0045): a machine built by hand is
    // given its id as a generated `sce-static` machine is, drawn.
    init {
        bindRootSessionId(null)
    }

    override fun firstEnabledTransition(
        state: ConfiguredState,
        event: ConfiguredEvent?,
    ): EnabledTransition<ConfiguredState, HistoryId>? = null

    override fun onEntry(state: ConfiguredState, isDefaultEntry: Boolean) {}

    override fun onExit(state: ConfiguredState) {}

    override fun executeTransitionContent(source: ConfiguredState, transitionIndex: Int) {}

    override fun resolveState(stateId: String): ConfiguredState? = ConfiguredState.takeIf { stateId == "only" }

    override fun stateIdOf(state: ConfiguredState): String = "only"

    override fun resolveEventByName(name: String): ConfiguredEvent? = ConfiguredEvent.Ping.takeIf { name == "ping" }

    override fun eventNameOf(event: ConfiguredEvent): String? = "ping"

    fun saveAt(wallNowMs: Long): SavedState = savedState(shape, linkedMapOf(), wallNowMs)

    fun restoreAt(saved: SavedState, wallNowMs: Long) {
        enterSaved(beginRestore(saved, shape), wallNowMs)
    }
}

class SavedConfigurationTest {

    private val savedAtMs = 1_700_000_000_000L

    /** A format-2 session of [sessionId] standing in `working`, taken with [features] on, running [children]. */
    private fun session(sessionId: String, features: List<String>?, children: List<SavedChild> = emptyList()) =
        SavedState(
            shape = "a".repeat(64),
            configuration = listOf("working"),
            current = "working",
            variables = mapOf("completed" to Json.Number("0")),
            invokes = SavedInvokes.Resumed(children),
            sessionId = sessionId,
            features = features,
        )

    /** A machine bound to a shape whose configuration is [features]. */
    private fun machine(features: List<String>?): ConfiguredMachine =
        ConfiguredMachine(
            SavedShape(
                first = "shape",
                revised = "revised shape",
                format2First = "shape",
                format2Revised = "revised shape",
                records = emptyList(),
                features = features,
            ),
        )

    private fun refused(saved: SavedState, built: List<String>?): String =
        assertFailsWith<StateRefusal> { SavedState.checkFeatures(saved, built) }.message ?: ""

    @Test
    fun `a saved state is read back as it was written, its features the last member of each session`() {
        val child = session("root.worker", listOf("tls"))
        val state = SavedState(
            shape = "b".repeat(64),
            configuration = listOf("working"),
            current = "working",
            variables = mapOf("completed" to Json.Number("2")),
            invokes = SavedInvokes.Resumed(listOf(SavedChild("worker", 1L, "worker", "worker", child))),
            sessionId = "root",
            invokeStarts = mapOf("worker" to 1L),
            revisions = mapOf("reading.taken" to SavedRevision(3, "0".repeat(64))),
            features = listOf("session-unicast", "tls"),
        )
        val text = state.toJson()
        assertEquals(state, SavedState.fromJson(text))
        // After `revisions`, and the last member of the root and of the child's session.
        assertTrue(text.endsWith(""","features":["session-unicast","tls"]}"""), text)
        assertTrue(text.indexOf("\"revisions\"") < text.lastIndexOf("\"features\""), text)
        assertTrue(text.contains(""","features":["tls"]}}"""), text)
        // `[]` is a configuration: the one that turns no feature on.
        val none = session("root", emptyList())
        assertTrue(none.toJson().endsWith(""","features":[]}"""), none.toJson())
        assertEquals(none, SavedState.fromJson(none.toJson()))
        // A machine of no product line records no configuration, and its text is
        // the one it was before configurations were recorded.
        val plain = session("root", null, listOf(SavedChild("worker", 1L, "worker", "worker", session("root.worker", null))))
        assertFalse(plain.toJson().contains("features"), plain.toJson())
        val back = SavedState.fromJson(plain.toJson())
        assertEquals(plain, back)
        assertNull(back.features)
        // Format 1 carries it too, after `external`.
        val format1 = """{"format":1,"shape":"s","configuration":["s"],"current":"s","variables":{},"history":{},"pending":[],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[],"features":["a"]}"""
        assertEquals(format1, SavedState.fromJson(format1).toJson())
    }

    @Test
    fun `a state of another configuration is refused by the features that differ`() {
        val taken = { features: List<String>? -> session("s", features) }
        SavedState.checkFeatures(taken(null), null)
        SavedState.checkFeatures(taken(emptyList()), emptyList())
        SavedState.checkFeatures(taken(listOf("a", "b")), listOf("a", "b"))
        assertEquals(
            "the saved state was taken with the features [a, b] on, and this machine is built " +
                "with [a, c]: [b] on only where it was saved, [c] on only in this build",
            refused(taken(listOf("a", "b")), listOf("a", "c")),
        )
        assertEquals(
            "the saved state was taken with the features [a] on, and this machine is built " +
                "with []: [a] on only where it was saved",
            refused(taken(listOf("a")), emptyList()),
        )
        assertEquals(
            "the saved state was taken with the features [] on, and this machine is built " +
                "with [a]: [a] on only in this build",
            refused(taken(emptyList()), listOf("a")),
        )
        assertEquals(
            "the saved state records no features, and this machine is built with the features [a] on",
            refused(taken(null), listOf("a")),
        )
        assertEquals(
            "the saved state was taken with the features [] on, and this machine is built " +
                "from a document of no feature model",
            refused(taken(emptyList()), null),
        )
        // Judged before the shape: a state of another build is refused for its
        // features even where its shape differs too, and one of this
        // configuration goes on to be judged by its shape.
        val shape = SavedShape(
            first = "c".repeat(64),
            revised = "d".repeat(64),
            format2First = "c".repeat(64),
            format2Revised = "d".repeat(64),
            records = emptyList(),
            features = listOf("a"),
        )
        val other = assertFailsWith<StateRefusal> { SavedState.upgrade(taken(listOf("b")), shape) }.message ?: ""
        assertTrue(other.startsWith("the saved state was taken with the features [b] on"), other)
        val same = assertFailsWith<StateRefusal> { SavedState.upgrade(taken(listOf("a")), shape) }.message ?: ""
        assertTrue(same.startsWith("the saved state is of a document of shape"), same)
        // What a restore reads on is the configuration it was taken in.
        val fitting = SavedShape(
            first = "a".repeat(64),
            revised = "e".repeat(64),
            format2First = "a".repeat(64),
            format2Revised = "e".repeat(64),
            records = emptyList(),
            features = listOf("a"),
        )
        assertEquals(listOf("a"), SavedState.upgrade(taken(listOf("a")), fitting).features)
    }

    @Test
    fun `the features of a saved state are a sorted set`() {
        val text = { features: String ->
            """{"format":1,"shape":"s","configuration":["s"],"current":"s","variables":{},"history":{},"pending":[],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[],"features":$features}"""
        }
        assertEquals(listOf("a", "b"), SavedState.fromJson(text("""["a","b"]""")).features)
        for (unordered in listOf("""["b","a"]""", """["a","a"]""")) {
            assertEquals(
                "'features' is not in order or names a feature twice; it is a set, sorted",
                assertFailsWith<StateRefusal>(unordered) { SavedState.fromJson(text(unordered)) }.message,
            )
        }
        assertEquals(
            "'features' is not an array",
            assertFailsWith<StateRefusal> { SavedState.fromJson(text("\"a\"")) }.message,
        )
        assertEquals(
            "'features' is not an array",
            assertFailsWith<StateRefusal> { SavedState.fromJson(text("null")) }.message,
        )
        assertEquals(
            "'features' is not a text",
            assertFailsWith<StateRefusal> { SavedState.fromJson(text("""["a",1]""")) }.message,
        )
        // The order is the order of the names' UTF-8 bytes, every backend's: a
        // character above U+FFFF follows U+E000, though its first UTF-16 unit is
        // below it.
        assertEquals(
            listOf("", "😀"),
            SavedState.fromJson(text("""["","😀"]""")).features,
        )
        assertFailsWith<StateRefusal> { SavedState.fromJson(text("""["😀",""]""")) }
    }

    @Test
    fun `a machine saves the configuration it was built for and a restore of another is refused`() {
        val saved = machine(listOf("a", "b")).let { sm ->
            try {
                sm.clock = ManualClock(0)
                sm.initialize()
                sm.saveAt(savedAtMs)
            } finally {
                sm.cleanup()
            }
        }
        assertEquals(listOf("a", "b"), saved.features)
        assertTrue(saved.toJson().endsWith(""","features":["a","b"]}"""), saved.toJson())
        val text = saved.toJson()

        // Another configuration: refused by the features that differ, and the
        // machine is left as it was, never started.
        val other = machine(listOf("a", "c"))
        try {
            other.clock = ManualClock(0)
            assertEquals(
                "the saved state was taken with the features [a, b] on, and this machine is built " +
                    "with [a, c]: [b] on only where it was saved, [c] on only in this build",
                assertFailsWith<StateRefusal> { other.restoreAt(SavedState.fromJson(text), savedAtMs) }.message,
            )
            assertFailsWith<StateRefusal> { other.saveAt(savedAtMs) }
        } finally {
            other.cleanup()
        }

        // A machine of no product line refuses it too.
        val plain = machine(null)
        try {
            plain.clock = ManualClock(0)
            assertEquals(
                "the saved state was taken with the features [a, b] on, and this machine is built " +
                    "from a document of no feature model",
                assertFailsWith<StateRefusal> { plain.restoreAt(SavedState.fromJson(text), savedAtMs) }.message,
            )
        } finally {
            plain.cleanup()
        }

        // The machine of the same configuration restores it and saves it again.
        val same = machine(listOf("a", "b"))
        try {
            same.clock = ManualClock(0)
            same.restoreAt(SavedState.fromJson(text), savedAtMs)
            val again = same.saveAt(savedAtMs)
            assertEquals(listOf("only"), again.configuration)
            assertEquals(listOf("a", "b"), again.features)
        } finally {
            same.cleanup()
        }
    }
}
