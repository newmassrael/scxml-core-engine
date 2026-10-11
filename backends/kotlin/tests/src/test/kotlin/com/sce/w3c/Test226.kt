// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test226.scxml:1
package com.sce.w3c

import com.sce.generated.test226.Test226Event
import com.sce.generated.test226.Test226State
import com.sce.generated.test226.Test226StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the invoke element is executed, the SCXML Processor MUST start a new logical instance of the external service specified in 'type' or 'typexpr', passing it the URL specified by 'src' or the data specified by content, or param.
@DisplayName("Test 226 -- W3C SCXML 6.4")
class Test226 : W3CTestBase<Test226State, Test226Event>() {
    override fun createStateMachine() = Test226StateMachine(createEngine())
    override val expectedPassState: Test226State = Test226State.Pass
}
