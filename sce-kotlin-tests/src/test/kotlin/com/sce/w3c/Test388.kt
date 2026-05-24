// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test388.scxml:1
package com.sce.w3c

import com.sce.generated.test388.Test388Event
import com.sce.generated.test388.Test388State
import com.sce.generated.test388.Test388StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.10: After the parent state has been visited for the first time, if a transition is executed that takes the history state as its target, the SCXML processor MUST behave as if the transition had taken the stored state configuration as its target.
@DisplayName("Test 388 -- W3C SCXML 3.10")
class Test388 : W3CTestBase<Test388State, Test388Event>() {
    override fun createStateMachine() = Test388StateMachine(createEngine())
    override val expectedPassState: Test388State = Test388State.Pass
}
