// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2c4f76809986b4347703e89a8e901379e8391f815371b53c5a7eecbe187e1cf5
// generated-at: 1781081955
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test453.scxml:1
package com.sce.w3c

import com.sce.generated.test453.Test453Event
import com.sce.generated.test453.Test453State
import com.sce.generated.test453.Test453StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel,  the SCXML Processor must accept any ECMAScript expression as a value expression.
@DisplayName("Test 453 -- W3C SCXML B.2")
class Test453 : W3CTestBase<Test453State, Test453Event>() {
    override fun createStateMachine() = Test453StateMachine(createEngine())
    override val expectedPassState: Test453State = Test453State.Pass
}
