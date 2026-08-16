// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 6b3d1716c5fe7bf441783d277357c458e7e14d8fc3f1d3e67e7f0181f437b229
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test436.scxml:1
package com.sce.w3c

import com.sce.generated.test436.Test436Event
import com.sce.generated.test436.Test436State
import com.sce.generated.test436.Test436StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.1: When the "datamodel" attribute of the scxml element has the value "null", the In() predicate must return 'true' if and only if that state is in the current state configuration.
@DisplayName("Test 436 -- W3C SCXML B.1")
class Test436 : W3CTestBase<Test436State, Test436Event>() {
    override fun createStateMachine() = Test436StateMachine()
    override val expectedPassState: Test436State = Test436State.Pass
}
