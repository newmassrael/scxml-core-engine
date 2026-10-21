// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test205.scxml:1
package com.sce.w3c

import com.sce.generated.test205.Test205Event
import com.sce.generated.test205.Test205State
import com.sce.generated.test205.Test205StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: The sending SCXML Interpreter MUST not alter the content of the send
@DisplayName("Test 205 -- W3C SCXML 6.2")
class Test205 : W3CTestBase<Test205State, Test205Event>() {
    override fun createStateMachine() = Test205StateMachine(createEngine())
    override val expectedPassState: Test205State = Test205State.Pass
}
