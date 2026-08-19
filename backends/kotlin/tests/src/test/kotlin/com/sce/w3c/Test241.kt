// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 5b0237a7a83721c40de92b1914fb5f3ab69530a228f19b8f33cd3af4e27ebf24
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
