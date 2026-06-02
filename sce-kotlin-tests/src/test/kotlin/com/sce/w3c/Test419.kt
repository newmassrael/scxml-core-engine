// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e8782a5c8351481fc8f6e7fcdb09caae80cbe9e47c6019dcf15afff703e3c3b3
// generated-at: 1780407549
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
