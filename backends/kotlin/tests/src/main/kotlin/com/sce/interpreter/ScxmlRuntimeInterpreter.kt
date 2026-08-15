// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML 6.4: Lightweight runtime SCXML interpreter for hybrid invoke children.
// C++ parity: matches StateMachine::createFromSCXMLString() + FileLoadingHelper::loadScxmlFile()

package com.sce.interpreter

import com.sce.runtime.*
import org.w3c.dom.Element
import org.w3c.dom.Node
import org.xml.sax.InputSource
import java.io.File
import java.io.StringReader
import javax.xml.parsers.DocumentBuilderFactory

// --- Dynamic State/Event types for runtime-parsed SCXML ---

data class DynState(val id: String) : State
data class DynEvent(val name: String) : Event

// --- Runtime SCXML Model ---

data class RuntimeStateInfo(
    val id: String,
    val isFinal: Boolean,
    val transitions: List<RuntimeTransition> = emptyList()
)

data class RuntimeTransition(
    val event: String,
    val target: String
)

/**
 * W3C SCXML 6.4: Runtime SCXML interpreter for hybrid invoke children.
 *
 * C++ parity: matches [StateMachine::createFromSCXMLString] behavior.
 * Parses SCXML content at runtime and executes the state machine dynamically,
 * enabling srcexpr/contentexpr to load arbitrary SCXML at runtime.
 *
 * Supported SCXML elements: scxml, state, final, transition.
 * Sufficient for all W3C SCXML invoke child patterns.
 */
class ScxmlRuntimeInterpreter private constructor(
    private val states: List<DynState>,
    private val stateInfos: Map<String, RuntimeStateInfo>,
    initialStateId: String,
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<DynState, DynEvent>(scriptEngine) {

    companion object {
        /**
         * C++ parity: StateMachine::createFromSCXMLString()
         * Creates interpreter from SCXML content string.
         */
        fun fromString(
            scxmlContent: String,
            scriptEngine: ScxmlScriptEngine? = null
        ): ScxmlRuntimeInterpreter {
            val (states, stateInfos, initialId) = parseScxml(scxmlContent)
            return ScxmlRuntimeInterpreter(states, stateInfos, initialId, scriptEngine)
        }

        /**
         * C++ parity: FileLoadingHelper::loadScxmlFile() + createFromSCXMLString()
         * Loads SCXML file with W3C-compliant path resolution, then creates interpreter.
         *
         * @param filePath File path (may include "file:" prefix)
         * @param basePath Parent SCXML directory for relative path resolution
         */
        fun fromFile(
            filePath: String,
            basePath: String = "",
            scriptEngine: ScxmlScriptEngine? = null
        ): ScxmlRuntimeInterpreter {
            val content = loadScxmlFile(filePath, basePath)
            return fromString(content, scriptEngine)
        }

        // W3C SCXML 6.4: File loading with URI prefix stripping and relative path resolution
        // C++ parity: FileLoadingHelper::loadScxmlFile() + normalizePath()
        private fun loadScxmlFile(filePath: String, basePath: String): String {
            var path = filePath
            if (path.startsWith("file://")) path = path.substring(7)
            else if (path.startsWith("file:")) path = path.substring(5)

            val file = if (File(path).isAbsolute) File(path) else File(basePath, path)
            require(file.exists()) {
                "W3C SCXML 6.4.3: Child SCXML file not found: ${file.absolutePath}"
            }
            return file.readText()
        }

        // Minimal SCXML parser — C++ parity: SCXMLParser::parseContent()
        private fun parseScxml(content: String): Triple<List<DynState>, Map<String, RuntimeStateInfo>, String> {
            val factory = DocumentBuilderFactory.newInstance()
            factory.isNamespaceAware = true
            val doc = factory.newDocumentBuilder().parse(InputSource(StringReader(content)))
            val root = doc.documentElement

            val explicitInitial = root.getAttribute("initial")?.takeIf { it.isNotEmpty() }
            val states = mutableListOf<DynState>()
            val stateInfos = mutableMapOf<String, RuntimeStateInfo>()
            var autoIdCounter = 0

            val children = root.childNodes
            for (i in 0 until children.length) {
                val node = children.item(i)
                if (node.nodeType != Node.ELEMENT_NODE) continue
                val elem = node as Element
                val localName = elem.localName ?: elem.tagName.substringAfterLast(":")

                if (localName != "state" && localName != "final") continue

                val id = elem.getAttribute("id")?.takeIf { it.isNotEmpty() }
                    ?: "${localName}_${autoIdCounter++}"
                val isFinal = localName == "final"

                states.add(DynState(id))

                // Parse child transitions
                val transitions = mutableListOf<RuntimeTransition>()
                val transChildren = elem.childNodes
                for (j in 0 until transChildren.length) {
                    val tNode = transChildren.item(j)
                    if (tNode.nodeType != Node.ELEMENT_NODE) continue
                    val tElem = tNode as Element
                    val tLocal = tElem.localName ?: tElem.tagName.substringAfterLast(":")
                    if (tLocal == "transition") {
                        val target = tElem.getAttribute("target") ?: ""
                        if (target.isNotEmpty()) {
                            transitions.add(RuntimeTransition(
                                event = tElem.getAttribute("event") ?: "",
                                target = target
                            ))
                        }
                    }
                }
                stateInfos[id] = RuntimeStateInfo(id, isFinal, transitions)
            }

            require(states.isNotEmpty()) { "W3C SCXML: No states found in SCXML content" }

            // W3C SCXML 3.6: Resolve initial state (explicit > first child in document order)
            val initialId = explicitInitial ?: states.first().id
            require(states.any { it.id == initialId }) {
                "W3C SCXML: Initial state '$initialId' not found in parsed states"
            }

            return Triple(states, stateInfos, initialId)
        }
    }

    override val initialState: DynState = states.first { it.id == initialStateId }

    override fun resolveState(stateId: String): DynState? =
        states.find { it.id == stateId }

    override fun stateIdOf(state: DynState): String = state.id

    override fun isAtomicState(state: DynState): Boolean = true

    override fun documentOrderOf(state: DynState): Int = states.indexOf(state)

    override fun resolveEventByName(name: String): DynEvent = DynEvent(name)

    override fun eventNameOf(event: DynEvent): String = event.name

    // W3C SCXML 3.12: Event-driven transition processing
    override fun processEvent(state: DynState, event: DynEvent): TransitionResult<DynState> {
        val info = stateInfos[state.id] ?: return TransitionResult.Ignored
        for (trans in info.transitions) {
            if (trans.event.isNotEmpty() && matchEvent(trans.event, event.name)) {
                val target = resolveState(trans.target) ?: continue
                return TransitionResult.External(target, state)
            }
        }
        return TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(state: DynState): TransitionResult<DynState> {
        val info = stateInfos[state.id] ?: return TransitionResult.Ignored
        for (trans in info.transitions) {
            if (trans.event.isEmpty()) {
                val target = resolveState(trans.target) ?: continue
                return TransitionResult.External(target, state)
            }
        }
        return TransitionResult.Ignored
    }

    // `pathChild` (§scxml-D) is unused here: this interpreter enters exactly the
    // states it is handed and never descends into defaults of its own, so it has
    // nothing to tell an ancestor entry from a target entry.
    override fun onEntry(state: DynState, pathChild: DynState?) {
        if (!activeStateIds.add(state.id)) return
        if (stateInfos[state.id]?.isFinal == true) {
            markFinalStateReached()
        }
    }

    override fun onExit(state: DynState) {
        activeStateIds.remove(state.id)
    }

    override fun executeTransitionActions(source: DynState, event: DynEvent?) {}

    // W3C SCXML 3.12.1: Event prefix matching
    // "error" matches "error", "error.execution", "error.communication" etc.
    private fun matchEvent(pattern: String, eventName: String): Boolean {
        if (pattern == "*") return true
        if (pattern == eventName) return true
        if (eventName.length > pattern.length && eventName[pattern.length] == '.' &&
            eventName.startsWith(pattern)) return true
        return false
    }
}
