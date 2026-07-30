// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425248
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test174.scxml:1
package com.sce.w3c

import com.sce.generated.test174.Test174Event
import com.sce.generated.test174.Test174State
import com.sce.generated.test174.Test174StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If 'typexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is evaluated and treat the result as if it had been entered as the value of 'type'.
@DisplayName("Test 174 -- W3C SCXML 6.2")
class Test174 : W3CTestBase<Test174State, Test174Event>() {
    override fun createStateMachine() = Test174StateMachine(createEngine())
    override val expectedPassState: Test174State = Test174State.Pass
}
