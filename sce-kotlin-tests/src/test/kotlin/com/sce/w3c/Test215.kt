// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 030a39123c8149accb30146fc4a4999b6e8826a330653d219a562116c552e0d8
// generated-at: 1781483328
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test215.scxml:1
package com.sce.w3c

import com.sce.generated.test215.Test215Event
import com.sce.generated.test215.Test215State
import com.sce.generated.test215.Test215StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the typeexpr attribute is present, the SCXML Processor MUST evaluate it when the parent invoke element is evaluated and treat the result as if it had been entered as the value of 'type'.
@DisplayName("Test 215 -- W3C SCXML 6.4")
class Test215 : W3CTestBase<Test215State, Test215Event>() {
    override fun createStateMachine() = Test215StateMachine(createEngine())
    override val expectedPassState: Test215State = Test215State.Pass
}
