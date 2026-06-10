// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2c4f76809986b4347703e89a8e901379e8391f815371b53c5a7eecbe187e1cf5
// generated-at: 1781081955
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test354.scxml:1
package com.sce.w3c

import com.sce.generated.test354.Test354Event
import com.sce.generated.test354.Test354State
import com.sce.generated.test354.Test354StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: The 'data' field of the event raised in the receiving session MUST contain a copy of the data specified in the 'namelist' attribute or in param or content elements in the sending session. The nature of the copy operation depends on the datamodel in question. However, the Processor MUST ensure that changes to the transmitted data in the receiving session do not affect the data in the sending session and vice-versa. The format of the 'data' field will depend on the datamodel of the receiving session.
@DisplayName("Test 354 -- W3C SCXML C.1")
class Test354 : W3CTestBase<Test354State, Test354Event>() {
    override fun createStateMachine() = Test354StateMachine(createEngine())
    override val expectedPassState: Test354State = Test354State.Pass
}
