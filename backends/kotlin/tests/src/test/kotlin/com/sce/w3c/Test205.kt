// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f8935a2b1ceca80a03ff3489cc9f8dcbccd8c2b85fc58c3b848403d6a2672153
// generated-at: 0
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
