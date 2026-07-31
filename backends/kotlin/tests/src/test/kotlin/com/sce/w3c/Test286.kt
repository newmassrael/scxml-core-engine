// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test286.scxml:1
package com.sce.w3c

import com.sce.generated.test286.Test286Event
import com.sce.generated.test286.Test286State
import com.sce.generated.test286.Test286StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.4: If the location expression of an assign does not denote a valid location in the datamodel the processor MUST place the error error.execution in the internal event queue.
@DisplayName("Test 286 -- W3C SCXML 5.4")
class Test286 : W3CTestBase<Test286State, Test286Event>() {
    override fun createStateMachine() = Test286StateMachine(createEngine())
    override val expectedPassState: Test286State = Test286State.Pass
}
