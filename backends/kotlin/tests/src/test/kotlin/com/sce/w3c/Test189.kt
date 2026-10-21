// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test189.scxml:1
package com.sce.w3c

import com.sce.generated.test189.Test189Event
import com.sce.generated.test189.Test189State
import com.sce.generated.test189.Test189StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: When using the scxml event i/o processor] If the target is the special term '#_internal', the Processor MUST add the event to the internal event queue of the sending session
@DisplayName("Test 189 -- W3C SCXML C.1")
class Test189 : W3CTestBase<Test189State, Test189Event>() {
    override fun createStateMachine() = Test189StateMachine()
    override val expectedPassState: Test189State = Test189State.Pass
}
