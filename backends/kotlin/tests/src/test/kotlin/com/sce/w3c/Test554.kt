// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test554.scxml:1
package com.sce.w3c

import com.sce.generated.test554.Test554Event
import com.sce.generated.test554.Test554State
import com.sce.generated.test554.Test554StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: if the evaluation of the invoke element's arguments arguments produces an error, the SCXML Processor MUST terminate the processing of the element without further action.
@DisplayName("Test 554 -- W3C SCXML 6.4")
class Test554 : W3CTestBase<Test554State, Test554Event>() {
    override fun createStateMachine() = Test554StateMachine(createEngine())
    override val expectedPassState: Test554State = Test554State.Pass
    override val timeoutMs: Long = 5000L
}
