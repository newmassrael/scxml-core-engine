// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 057f3064c2c620977191e86f67c1d505edec850a0d81b50b27d4b101952af703
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test241.scxml:1
package com.sce.w3c

import com.sce.generated.test241.Test241Event
import com.sce.generated.test241.Test241State
import com.sce.generated.test241.Test241StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Invoked services MUST treat values specified by param and namelist identically.
@DisplayName("Test 241 -- W3C SCXML 6.4")
class Test241 : W3CTestBase<Test241State, Test241Event>() {
    override fun createStateMachine() = Test241StateMachine(createEngine())
    override val expectedPassState: Test241State = Test241State.Pass
    override val timeoutMs: Long = 5000L
}
