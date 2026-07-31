// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
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
