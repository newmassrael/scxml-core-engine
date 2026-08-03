// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 1648c68c7039bcd2d9f4b6a29e08b82f1fcf3cd79ecb3462ff4016858820460c
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test322.scxml:1
package com.sce.w3c

import com.sce.generated.test322.Test322Event
import com.sce.generated.test322.Test322State
import com.sce.generated.test322.Test322StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST keep the _sessionid variable bound to the system-generated id until the session terminates.
@DisplayName("Test 322 -- W3C SCXML 5.10")
class Test322 : W3CTestBase<Test322State, Test322Event>() {
    override fun createStateMachine() = Test322StateMachine(createEngine())
    override val expectedPassState: Test322State = Test322State.Pass
}
