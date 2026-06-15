// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 030a39123c8149accb30146fc4a4999b6e8826a330653d219a562116c552e0d8
// generated-at: 1781483328
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test324.scxml:1
package com.sce.w3c

import com.sce.generated.test324.Test324Event
import com.sce.generated.test324.Test324State
import com.sce.generated.test324.Test324StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST keep the _name variable bound to the value of the 'name' attribute of the scxml element until the session terminates.
@DisplayName("Test 324 -- W3C SCXML 5.10")
class Test324 : W3CTestBase<Test324State, Test324Event>() {
    override fun createStateMachine() = Test324StateMachine(createEngine())
    override val expectedPassState: Test324State = Test324State.Pass
}
