// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: beb72c3a9cb76e61aa4916ff585cb6a1d22e66c189bf8cc96c5023dec391d982
// generated-at: 1780379958
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
