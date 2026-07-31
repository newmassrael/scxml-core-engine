// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test153.scxml:1
package com.sce.w3c

import com.sce.generated.test153.Test153Event
import com.sce.generated.test153.Test153State
import com.sce.generated.test153.Test153StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: When evaluating foreach, the SCXML processor MUST start with the first item in the collection and proceed to the last item in the iteration order that is defined for the collection. For each item in the collection in turn, the processor MUST assign it to the item variable.
@DisplayName("Test 153 -- W3C SCXML 4.6")
class Test153 : W3CTestBase<Test153State, Test153Event>() {
    override fun createStateMachine() = Test153StateMachine(createEngine())
    override val expectedPassState: Test153State = Test153State.Pass
}
