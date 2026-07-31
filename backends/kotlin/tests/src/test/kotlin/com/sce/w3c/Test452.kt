// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test452.scxml:1
package com.sce.w3c

import com.sce.generated.test452.Test452Event
import com.sce.generated.test452.Test452State
import com.sce.generated.test452.Test452StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, the SCXML Processor must accept any ECMAScript left-hand-side expression as a location expression.
@DisplayName("Test 452 -- W3C SCXML B.2")
class Test452 : W3CTestBase<Test452State, Test452Event>() {
    override fun createStateMachine() = Test452StateMachine(createEngine())
    override val expectedPassState: Test452State = Test452State.Pass
}
