// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b987ea47cf7b98cc29f6a07fbb829bd85b24bd9991a16621d5e7458fb0482788
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test230.scxml:1
package com.sce.w3c

import com.sce.generated.test230.Test230Event
import com.sce.generated.test230.Test230State
import com.sce.generated.test230.Test230StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the SCXML Processor autoforwards an event to the invoked process, all the fields specified in 5.11.1 The Internal Structure of Events MUST have the same values in the forwarded copy of the event
@DisplayName("Test 230 -- W3C SCXML 6.4")
class Test230 : W3CTestBase<Test230State, Test230Event>() {
    override fun createStateMachine() = Test230StateMachine(createEngine())
    override val expectedPassState: Test230State = Test230State.Final
    override val timeoutMs: Long = 5000L
}
