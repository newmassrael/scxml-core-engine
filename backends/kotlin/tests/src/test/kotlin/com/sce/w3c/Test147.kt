// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425248
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test147.scxml:1
package com.sce.w3c

import com.sce.generated.test147.Test147Event
import com.sce.generated.test147.Test147State
import com.sce.generated.test147.Test147StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.3: When the if element is executed, the SCXML processor MUST execute the first partition in document order that is defined by a tag whose 'cond' attribute evaluates to true, if there is one.
@DisplayName("Test 147 -- W3C SCXML 4.3")
class Test147 : W3CTestBase<Test147State, Test147Event>() {
    override fun createStateMachine() = Test147StateMachine(createEngine())
    override val expectedPassState: Test147State = Test147State.Pass
}
