// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c496f893fb4def171deba817f047a2a335356d181c631fa74825a157a7412c3e
// generated-at: 1784370263
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test527.scxml:1
package com.sce.w3c

import com.sce.generated.test527.Test527Event
import com.sce.generated.test527.Test527State
import com.sce.generated.test527.Test527StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.6: When the SCXML Processor evaluates the content element, if the 'expr' value expression is present, the Processor MUST evaluate it and use the result as the output of the content element.
@DisplayName("Test 527 -- W3C SCXML 5.6")
class Test527 : W3CTestBase<Test527State, Test527Event>() {
    override fun createStateMachine() = Test527StateMachine(createEngine())
    override val expectedPassState: Test527State = Test527State.Pass
}
