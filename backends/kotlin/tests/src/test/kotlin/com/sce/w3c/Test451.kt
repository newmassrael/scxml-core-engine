// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 140c4d555915ab51dfdb5b562572972b7994e3bced0046a696f7c65e279b5a12
// generated-at: 1785462747
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test451.scxml:1
package com.sce.w3c

import com.sce.generated.test451.Test451Event
import com.sce.generated.test451.Test451State
import com.sce.generated.test451.Test451StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, the SCXML Processor must add an ECMAScript
@DisplayName("Test 451 -- W3C SCXML B.2")
class Test451 : W3CTestBase<Test451State, Test451Event>() {
    override fun createStateMachine() = Test451StateMachine()
    override val expectedPassState: Test451State = Test451State.Pass
}
