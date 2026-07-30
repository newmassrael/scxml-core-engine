// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425248
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test326.scxml:1
package com.sce.w3c

import com.sce.generated.test326.Test326Event
import com.sce.generated.test326.Test326State
import com.sce.generated.test326.Test326StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST keep the _ioprocessors variable bound to its set of values until the session terminates.
@DisplayName("Test 326 -- W3C SCXML 5.10")
class Test326 : W3CTestBase<Test326State, Test326Event>() {
    override fun createStateMachine() = Test326StateMachine(createEngine())
    override val expectedPassState: Test326State = Test326State.Pass
}
