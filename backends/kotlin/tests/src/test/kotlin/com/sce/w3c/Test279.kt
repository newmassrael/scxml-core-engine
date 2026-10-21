// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test279.scxml:1
package com.sce.w3c

import com.sce.generated.test279.Test279Event
import com.sce.generated.test279.Test279State
import com.sce.generated.test279.Test279StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: When 'binding' attribute on the scxml element is assigned the value "early" (the default), the SCXML Processor MUST create all data elements and assign their initial values at document initialization time.
@DisplayName("Test 279 -- W3C SCXML 5.3")
class Test279 : W3CTestBase<Test279State, Test279Event>() {
    override fun createStateMachine() = Test279StateMachine(createEngine())
    override val expectedPassState: Test279State = Test279State.Pass
}
