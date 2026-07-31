// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0
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
