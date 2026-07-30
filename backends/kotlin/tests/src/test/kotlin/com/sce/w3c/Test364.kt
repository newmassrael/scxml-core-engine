// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425248
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test364.scxml:1
package com.sce.w3c

import com.sce.generated.test364.Test364Event
import com.sce.generated.test364.Test364State
import com.sce.generated.test364.Test364StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.3: Definition: The default initial state(s) of a compound state are those specified by the 'initial' attribute or initial element, if either is present. Otherwise it is the state's first child state in document order. If a compound state is entered either as an initial state or as the target of a transition (i.e. and no descendent of it is specified), then the SCXML Processor MUST enter the default initial state(s) after it enters the parent state.
@DisplayName("Test 364 -- W3C SCXML 3.3")
class Test364 : W3CTestBase<Test364State, Test364Event>() {
    override fun createStateMachine() = Test364StateMachine()
    override val expectedPassState: Test364State = Test364State.Pass
}
