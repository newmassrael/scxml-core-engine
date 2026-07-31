// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: c6c9654e14987bf9fee21998d111ca1385c48c09f2deb9cc862525d124525214
// generated-at: 1785480867
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test528.scxml:1
package com.sce.w3c

import com.sce.generated.test528.Test528Event
import com.sce.generated.test528.Test528State
import com.sce.generated.test528.Test528StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.6: f the evaluation of 'expr' produces an error, the Processor MUST place error.execution in the internal event queue and use the empty string as the output of the content element.
@DisplayName("Test 528 -- W3C SCXML 5.6")
class Test528 : W3CTestBase<Test528State, Test528Event>() {
    override fun createStateMachine() = Test528StateMachine(createEngine())
    override val expectedPassState: Test528State = Test528State.Pass
}
