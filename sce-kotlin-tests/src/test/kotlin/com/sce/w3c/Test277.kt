// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2c4f76809986b4347703e89a8e901379e8391f815371b53c5a7eecbe187e1cf5
// generated-at: 1781081955
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test277.scxml:1
package com.sce.w3c

import com.sce.generated.test277.Test277Event
import com.sce.generated.test277.Test277State
import com.sce.generated.test277.Test277StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: If the value specified for a data element (by 'src', children, or the environment) is not a legal data value, the SCXML Processor MUST raise place error.execution in the internal event queue and MUST create an empty data element in the data model with the specified id.
@DisplayName("Test 277 -- W3C SCXML 5.3")
class Test277 : W3CTestBase<Test277State, Test277Event>() {
    override fun createStateMachine() = Test277StateMachine(createEngine())
    override val expectedPassState: Test277State = Test277State.Pass
}
