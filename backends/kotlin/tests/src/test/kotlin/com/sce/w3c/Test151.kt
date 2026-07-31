// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test151.scxml:1
package com.sce.w3c

import com.sce.generated.test151.Test151Event
import com.sce.generated.test151.Test151State
import com.sce.generated.test151.Test151StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: In the foreach element, if 'index' is present, the SCXML processor MUST declare a new variable if the one specified by 'index' is not already defined.
@DisplayName("Test 151 -- W3C SCXML 4.6")
class Test151 : W3CTestBase<Test151State, Test151Event>() {
    override fun createStateMachine() = Test151StateMachine(createEngine())
    override val expectedPassState: Test151State = Test151State.Pass
}
