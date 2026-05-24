// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
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
