// SCE-GENERATED — DO NOT EDIT
// source-hash: 7c990b384ae6d27b45cff45f6fb75ecde882d112d0f07d342d547b178e6a4257
// template-hash: 0df7c3dd89bf1ab35c62dca175cae2bb2e377b70fda63f4fb76009a06edcd3df
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/xml_data_is_a_dom_tree/xml_data_is_a_dom_tree.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: xml_data_is_a_dom_tree.scxml:44 :: _machine

package com.sce.integration.xml_data_is_a_dom_tree

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface XmlDataIsADomTreeState : State {
    data object NotADocument : XmlDataIsADomTreeState
    data object NoText : XmlDataIsADomTreeState
    data object Reading : XmlDataIsADomTreeState
    data object ReadingText : XmlDataIsADomTreeState
    data object Settled : XmlDataIsADomTreeState
    data object Traversing : XmlDataIsADomTreeState
    data object WrongTree : XmlDataIsADomTreeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface XmlDataIsADomTreeEvent : Event {
    sealed interface Error : XmlDataIsADomTreeEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class XmlDataIsADomTreeStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<XmlDataIsADomTreeState, XmlDataIsADomTreeEvent>(scriptEngine) {

    override val initialState: XmlDataIsADomTreeState = XmlDataIsADomTreeState.Reading

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): XmlDataIsADomTreeState? = when (stateId) {
        "notADocument" -> XmlDataIsADomTreeState.NotADocument
        "noText" -> XmlDataIsADomTreeState.NoText
        "reading" -> XmlDataIsADomTreeState.Reading
        "readingText" -> XmlDataIsADomTreeState.ReadingText
        "settled" -> XmlDataIsADomTreeState.Settled
        "traversing" -> XmlDataIsADomTreeState.Traversing
        "wrongTree" -> XmlDataIsADomTreeState.WrongTree
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: XmlDataIsADomTreeState): String = when (state) {
        is XmlDataIsADomTreeState.NotADocument -> "notADocument"
        is XmlDataIsADomTreeState.NoText -> "noText"
        is XmlDataIsADomTreeState.Reading -> "reading"
        is XmlDataIsADomTreeState.ReadingText -> "readingText"
        is XmlDataIsADomTreeState.Settled -> "settled"
        is XmlDataIsADomTreeState.Traversing -> "traversing"
        is XmlDataIsADomTreeState.WrongTree -> "wrongTree"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: XmlDataIsADomTreeState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: XmlDataIsADomTreeState): Int = when (state) {
        is XmlDataIsADomTreeState.NotADocument -> 4
        is XmlDataIsADomTreeState.NoText -> 6
        is XmlDataIsADomTreeState.Reading -> 0
        is XmlDataIsADomTreeState.ReadingText -> 2
        is XmlDataIsADomTreeState.Settled -> 3
        is XmlDataIsADomTreeState.Traversing -> 1
        is XmlDataIsADomTreeState.WrongTree -> 5
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): XmlDataIsADomTreeEvent? = when (name) {
        "error.execution" -> XmlDataIsADomTreeEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: XmlDataIsADomTreeEvent): String? = when (event) {
        is XmlDataIsADomTreeEvent.Error.Execution -> "error.execution"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // §scxml-C-1-1 / §scxml-C-2-3: the `_ioprocessors` entries come from the
        // same helper every other backend uses, so a machine reads the same
        // entry names and the same addresses whichever one runs it.
        engine.setupSystemVariables(
            sid,
            "xml_data_is_a_dom_tree",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML B.2: Initialize variable 'doc' with inline content (C++ parseEventData pattern)
        try {
            val initResult_doc = engine.parseDataValue(sid, "<books xmlns=\"\" count=\"2\">\n        <book title=\"t1\">first</book>\n        <book title=\"t2\"></book>\n      </books>")
            engine.setVariable(sid, "doc", initResult_doc)
        } catch (e: Exception) {
            raisePlatformError(XmlDataIsADomTreeEvent.Error.Execution, "<data id='doc'> content failed to initialise")
        }




        // W3C SCXML 6.4: Apply pending invoke params from parent
        // Only set params matching child's declared datamodel variables (C++ DatamodelValidationHelper)
        if (pendingInvokeParams.isNotEmpty()) {
            for ((pName, pValue) in pendingInvokeParams) {
                if (engine.hasVariable(sid, pName)) {
                    try { engine.setVariable(sid, pName, pValue) } catch (_: Exception) {}
                }
            }
            pendingInvokeParams = emptyMap()
        }

        scriptEngineInitialized = true
    }

    // W3C SCXML 5.9: Guard evaluation with error.execution on failure
    private fun safeEvaluateGuard(guardExpr: String): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(XmlDataIsADomTreeEvent.Error.Execution, "a <transition> cond failed to evaluate")
            false
        }
    }

    // W3C SCXML B.2: the value of an inline `<content>` body, serialized
    // for transport.
    //
    // The reading is decided at build time — `source` is already the
    // expression or string literal the clause's ordered readings give —
    // and this evaluates it *here*, at send time, rather than handing the
    // expression to whatever reads `_event.data` later. That distinction
    // is not academic: the two engines this backend runs on disagree
    // about what a data string is. QuickJS tries a JS evaluation before
    // falling back; Rhino goes straight from JSON to the normalized
    // string, so an expression handed to it arrives as its own source
    // text. `JSON.stringify` is what both of them can read back, and it
    // is the same shape the C++ backend transports.
    private fun evaluateSendContent(source: String): String {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateExpr(sid, "JSON.stringify((" + source + "))")?.toString() ?: ""
        } catch (e: Exception) {
            raisePlatformError(XmlDataIsADomTreeEvent.Error.Execution, "an expression could not be serialised to JSON")
            ""
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    private fun executeAssign(location: String, expr: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raisePlatformError(XmlDataIsADomTreeEvent.Error.Execution, "<assign> failed")
        }
    }

    // W3C SCXML 5.8: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raisePlatformError(XmlDataIsADomTreeEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: XmlDataIsADomTreeEvent) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        val eventName = eventNameOf(event) ?: return
        val meta = currentEventMetadata
        // W3C SCXML 5.10.1: C++ classifyEventType — platform events override type
        val effectiveType = when {
            eventName.startsWith("done.") || eventName.startsWith("error.") -> "platform"
            else -> meta.type
        }
        // W3C SCXML 5.10.1: C++ pattern — origin/origintype only for external events
        // Internal events (<raise>) have empty origin; external events (<send>) have session ID
        // W3C SCXML C.1: `_event.origin` is the sender's published
        // `_ioprocessors` location, not its bare session id — and this is the
        // one place that publishes `_event` to the document, so this is where
        // the id becomes a location. The engine keeps the bare id in
        // `EventMetadata.origin` because its session-keyed lookups (`<finalize>`
        // dispatch, cancelled-invoke filtering) match on it; converting at the
        // raise would make one value serve two consumers that need different
        // spellings. The conversion itself lives in
        // `com.sce.runtime.IoProcessors.publishedOrigin`, the port of the
        // `IOProcessorHelper::publishedOrigin` the C++ engines share: a second
        // spelling of the rule is how the backends would stop agreeing.
        val effectiveOrigin = com.sce.runtime.IoProcessors.publishedOrigin(
            if (meta.type == "external") meta.origin.ifEmpty { scriptSessionId ?: "" } else meta.origin
        )
        val effectiveOriginType = if (meta.type == "external") meta.originType.ifEmpty { "http://www.w3.org/TR/scxml/#SCXMLEventProcessor" } else meta.originType
        // §scxml-B-2-8-1: the binding answers which rung the payload got, and
        // that answer used to end here. The ladder decided between a DOM, a
        // value and a space-normalized string, and the decision was dropped —
        // so a payload that announced structure and would not parse reached
        // the document as raw characters, every `_event.data.<field>` read
        // empty, and nothing anywhere could say so.
        //
        // Recorded on the spot rather than returned up: this class extends
        // `StateMachineEngine`, so the frame that binds already holds both the
        // reading and the event it belongs to — which is the pairing the count
        // needs.
        val payloadReading = engine.setCurrentEvent(
            sid,
            com.sce.runtime.SetCurrentEventArgs(
                name = eventName,
                data = meta.data,
                type = effectiveType,
                sendId = meta.sendId,
                origin = effectiveOrigin,
                originType = effectiveOriginType,
                invokeId = meta.invokeId
            )
        )
        notePayloadReading(event, payloadReading)
    }


    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: XmlDataIsADomTreeState,
        event: XmlDataIsADomTreeEvent
    ): TransitionResult<XmlDataIsADomTreeState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: XmlDataIsADomTreeState
    ): TransitionResult<XmlDataIsADomTreeState> = when (state) {
        is XmlDataIsADomTreeState.Reading -> processNullReading()
        is XmlDataIsADomTreeState.ReadingText -> processNullReadingText()
        is XmlDataIsADomTreeState.Traversing -> processNullTraversing()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullReading(
    ): TransitionResult<XmlDataIsADomTreeState> = when {
        safeEvaluateGuard("doc.nodeType === 9 && doc.nodeName === '#document' && doc.documentElement.tagName === 'books' && doc.hasAttribute('count')") -> TransitionResult.External(XmlDataIsADomTreeState.Traversing, XmlDataIsADomTreeState.Reading)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(XmlDataIsADomTreeState.NotADocument, XmlDataIsADomTreeState.Reading)
    }

    private fun processNullReadingText(
    ): TransitionResult<XmlDataIsADomTreeState> = when {
        safeEvaluateGuard("doc.documentElement.firstChild.firstChild.nodeType === 3 && doc.documentElement.firstChild.firstChild.nodeValue === 'first' && doc.documentElement.textContent === 'first' && doc.documentElement.lastChild.hasChildNodes() === false") -> TransitionResult.External(XmlDataIsADomTreeState.Settled, XmlDataIsADomTreeState.ReadingText)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(XmlDataIsADomTreeState.NoText, XmlDataIsADomTreeState.ReadingText)
    }

    private fun processNullTraversing(
    ): TransitionResult<XmlDataIsADomTreeState> = when {
        safeEvaluateGuard("doc.documentElement.childNodes.length === 2 && doc.documentElement.firstChild.getAttribute('title') === 't1' && doc.documentElement.lastChild.getAttribute('title') === 't2' && doc.documentElement.lastChild.previousSibling.getAttribute('title') === 't1' && doc.documentElement.firstChild.parentNode.tagName === 'books'") -> TransitionResult.External(XmlDataIsADomTreeState.ReadingText, XmlDataIsADomTreeState.Traversing)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(XmlDataIsADomTreeState.WrongTree, XmlDataIsADomTreeState.Traversing)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: xml_data_is_a_dom_tree.scxml:44 :: _machine
    override fun onEntry(state: XmlDataIsADomTreeState, pathChild: XmlDataIsADomTreeState?) {
        when (state) {
            is XmlDataIsADomTreeState.NotADocument -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:86 :: notADocument :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("notADocument")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is XmlDataIsADomTreeState.NoText -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:88 :: noText :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("noText")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is XmlDataIsADomTreeState.Reading -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:63 :: reading :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("reading")) return
            }
            is XmlDataIsADomTreeState.ReadingText -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:79 :: readingText :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("readingText")) return
            }
            is XmlDataIsADomTreeState.Settled -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:85 :: settled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("settled")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is XmlDataIsADomTreeState.Traversing -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:71 :: traversing :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("traversing")) return
            }
            is XmlDataIsADomTreeState.WrongTree -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:87 :: wrongTree :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("wrongTree")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: xml_data_is_a_dom_tree.scxml:44 :: _machine
    override fun onExit(state: XmlDataIsADomTreeState) {
        when (state) {
            is XmlDataIsADomTreeState.NotADocument -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:86 :: notADocument :: _state_body
                activeStateIds.remove("notADocument")
            }
            is XmlDataIsADomTreeState.NoText -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:88 :: noText :: _state_body
                activeStateIds.remove("noText")
            }
            is XmlDataIsADomTreeState.Reading -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:63 :: reading :: _state_body
                activeStateIds.remove("reading")
            }
            is XmlDataIsADomTreeState.ReadingText -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:79 :: readingText :: _state_body
                activeStateIds.remove("readingText")
            }
            is XmlDataIsADomTreeState.Settled -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:85 :: settled :: _state_body
                activeStateIds.remove("settled")
            }
            is XmlDataIsADomTreeState.Traversing -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:71 :: traversing :: _state_body
                activeStateIds.remove("traversing")
            }
            is XmlDataIsADomTreeState.WrongTree -> {
                // SCE-MAP: xml_data_is_a_dom_tree.scxml:87 :: wrongTree :: _state_body
                activeStateIds.remove("wrongTree")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: xml_data_is_a_dom_tree.scxml:44 :: _machine
    override fun executeTransitionActions(
        source: XmlDataIsADomTreeState,
        event: XmlDataIsADomTreeEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
