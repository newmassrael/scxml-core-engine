// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test419.scxml:1
package com.sce.w3c

import com.sce.generated.test419.Test419Event
import com.sce.generated.test419.Test419State
import com.sce.generated.test419.Test419StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: After checking the state configuration, the Processor MUST select the optimal transition set enabled by NULL in the current configuration.  If the [optimal transition] set [enabled by NULL in the current configuration] is not
@DisplayName("Test 419 -- W3C SCXML 3.13")
class Test419 : W3CTestBase<Test419State, Test419Event>() {
    override fun createStateMachine() = Test419StateMachine()
    override val expectedPassState: Test419State = Test419State.Pass
}
