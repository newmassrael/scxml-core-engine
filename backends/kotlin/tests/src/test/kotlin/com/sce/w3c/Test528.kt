// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
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
