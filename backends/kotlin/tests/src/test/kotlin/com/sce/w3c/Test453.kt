// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test453.scxml:1
package com.sce.w3c

import com.sce.generated.test453.Test453Event
import com.sce.generated.test453.Test453State
import com.sce.generated.test453.Test453StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel,  the SCXML Processor must accept any ECMAScript expression as a value expression.
@DisplayName("Test 453 -- W3C SCXML B.2")
class Test453 : W3CTestBase<Test453State, Test453Event>() {
    override fun createStateMachine() = Test453StateMachine(createEngine())
    override val expectedPassState: Test453State = Test453State.Pass
}
