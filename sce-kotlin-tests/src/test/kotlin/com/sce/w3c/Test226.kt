// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2c4f76809986b4347703e89a8e901379e8391f815371b53c5a7eecbe187e1cf5
// generated-at: 1781081955
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test226.scxml:1
package com.sce.w3c

import com.sce.generated.test226.Test226Event
import com.sce.generated.test226.Test226State
import com.sce.generated.test226.Test226StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the invoke element is executed, the SCXML Processor MUST start a new logical instance of the external service specified in 'type' or 'typexpr', passing it the URL specified by 'src' or the data specified by content, or param.
@DisplayName("Test 226 -- W3C SCXML 6.4")
class Test226 : W3CTestBase<Test226State, Test226Event>() {
    override fun createStateMachine() = Test226StateMachine(createEngine())
    override val expectedPassState: Test226State = Test226State.Pass
}
