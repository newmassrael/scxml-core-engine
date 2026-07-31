// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test329.scxml:1
package com.sce.w3c

import com.sce.generated.test329.Test329Event
import com.sce.generated.test329.Test329State
import com.sce.generated.test329.Test329StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST cause any attempt to change the value of a system variable to fail.
@DisplayName("Test 329 -- W3C SCXML 5.10")
class Test329 : W3CTestBase<Test329State, Test329Event>() {
    override fun createStateMachine() = Test329StateMachine(createEngine())
    override val expectedPassState: Test329State = Test329State.Pass
}
