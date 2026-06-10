// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2c4f76809986b4347703e89a8e901379e8391f815371b53c5a7eecbe187e1cf5
// generated-at: 1781081955
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test198.scxml:1
package com.sce.w3c

import com.sce.generated.test198.Test198Event
import com.sce.generated.test198.Test198State
import com.sce.generated.test198.Test198StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If neither the 'type' nor the 'typeexpr' is defined, the SCXML Processor MUST assume the default value of http://www.w3.org/TR/scxml/#SCXMLEventProcessor.
@DisplayName("Test 198 -- W3C SCXML 6.2")
class Test198 : W3CTestBase<Test198State, Test198Event>() {
    override fun createStateMachine() = Test198StateMachine(createEngine())
    override val expectedPassState: Test198State = Test198State.Pass
}
