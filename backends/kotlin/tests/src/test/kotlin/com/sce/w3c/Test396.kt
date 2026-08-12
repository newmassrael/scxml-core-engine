// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 74ba562b33766da248288b5dadec1e79a0ebb46a66e38786f6a7a4b2ccd653e3
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test396.scxml:1
package com.sce.w3c

import com.sce.generated.test396.Test396Event
import com.sce.generated.test396.Test396State
import com.sce.generated.test396.Test396StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.12: The SCXML processor MUST use this same name value [the one reflected in the event variable] to match against the 'event' attribute of transitions.
@DisplayName("Test 396 -- W3C SCXML 3.12")
class Test396 : W3CTestBase<Test396State, Test396Event>() {
    override fun createStateMachine() = Test396StateMachine()
    override val expectedPassState: Test396State = Test396State.Pass
}
