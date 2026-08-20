// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 01d4ae2083bec7e32e332b36f0feb0c22f9503210f70693517ba1b7aa0094003
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test338.scxml:1
package com.sce.w3c

import com.sce.generated.test338.Test338Event
import com.sce.generated.test338.Test338State
import com.sce.generated.test338.Test338StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: If an event is generated from an invoked child process, the Processor MUST set the invokeid field to the invoke id of the invocation that triggered the child process.
@DisplayName("Test 338 -- W3C SCXML 5.10")
class Test338 : W3CTestBase<Test338State, Test338Event>() {
    override fun createStateMachine() = Test338StateMachine(createEngine())
    override val expectedPassState: Test338State = Test338State.Pass
}
