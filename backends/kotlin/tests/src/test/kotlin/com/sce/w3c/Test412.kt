// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 140c4d555915ab51dfdb5b562572972b7994e3bced0046a696f7c65e279b5a12
// generated-at: 1785462747
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test412.scxml:1
package com.sce.w3c

import com.sce.generated.test412.Test412Event
import com.sce.generated.test412.Test412State
import com.sce.generated.test412.Test412StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If the state is a default entry state and has an initial child, the SCXML Processor MUST then [after doing the active state add and the onentry handlers] execute the executable content in the initial child's transition.
@DisplayName("Test 412 -- W3C SCXML 3.13")
class Test412 : W3CTestBase<Test412State, Test412Event>() {
    override fun createStateMachine() = Test412StateMachine()
    override val expectedPassState: Test412State = Test412State.Pass
    override val timeoutMs: Long = 5000L
}
