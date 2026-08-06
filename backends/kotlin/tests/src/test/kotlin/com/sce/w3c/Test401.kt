// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 7cd07f2c974b616900d2b201907d23253ba7d2b7e90840149b8c3f98eea7706a
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test401.scxml:1
package com.sce.w3c

import com.sce.generated.test401.Test401Event
import com.sce.generated.test401.Test401State
import com.sce.generated.test401.Test401StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.12: The processor MUST place these [error] events in the internal event queue.
@DisplayName("Test 401 -- W3C SCXML 3.12")
class Test401 : W3CTestBase<Test401State, Test401Event>() {
    override fun createStateMachine() = Test401StateMachine(createEngine())
    override val expectedPassState: Test401State = Test401State.Pass
}
