// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 465642caa5c7ae5f006b7e4c3302ebaf26878f27c380322c3cf9d87ca35b0ee6
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test577.scxml:1
package com.sce.w3c

import com.sce.generated.test577.Test577Event
import com.sce.generated.test577.Test577State
import com.sce.generated.test577.Test577StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If neither the 'target' nor the 'targetexpr' attribute is specified, the SCXML Processor MUST add the event error.communication to the internal event queue of the sending session.
@DisplayName("Test 577 -- W3C SCXML C.2")
class Test577 : W3CTestBase<Test577State, Test577Event>() {
    override fun createStateMachine() = Test577StateMachine()
    override val expectedPassState: Test577State = Test577State.Pass
}
