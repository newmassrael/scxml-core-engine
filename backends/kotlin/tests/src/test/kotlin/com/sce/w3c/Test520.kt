// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test520.scxml:1
package com.sce.w3c

import com.sce.generated.test520.Test520Event
import com.sce.generated.test520.Test520State
import com.sce.generated.test520.Test520StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If a content child is present, the SCXML Processor MUST use its value as the body of the message.
@DisplayName("Test 520 -- W3C SCXML C.2")
class Test520 : W3CHttpTestBase<Test520State, Test520Event>() {
    override fun createStateMachine() = Test520StateMachine(createEngine())
    override val expectedPassState: Test520State = Test520State.Pass
}
