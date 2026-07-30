// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425248
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test287.scxml:1
package com.sce.w3c

import com.sce.generated.test287.Test287Event
import com.sce.generated.test287.Test287State
import com.sce.generated.test287.Test287StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.4: If the location expression of an assign denotes a valid location in the datamodel and if the value specified by 'expr' is a legal value for the location specified, the processor MUST place the specified value at the specified location.
@DisplayName("Test 287 -- W3C SCXML 5.4")
class Test287 : W3CTestBase<Test287State, Test287Event>() {
    override fun createStateMachine() = Test287StateMachine(createEngine())
    override val expectedPassState: Test287State = Test287State.Pass
}
