// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: bf9f012bd8272e352f46f4d8064cf0cf3b743ab6fffdf8c941cc03f3254cb15f
// generated-at: 0
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
