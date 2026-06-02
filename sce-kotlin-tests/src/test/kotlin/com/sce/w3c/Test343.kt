// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bc7b5b1dd90f65e6c3a4df2e3c4223cf8922d7e6b2d5d124b66683d16074cb6e
// generated-at: 1780362263
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test343.scxml:1
package com.sce.w3c

import com.sce.generated.test343.Test343Event
import com.sce.generated.test343.Test343State
import com.sce.generated.test343.Test343StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.7: If the 'location' attribute on a param element does not refer to a valid location in the data model, or if the evaluation of the 'expr' produces an error, the processor MUST ignore the name and value.
@DisplayName("Test 343 -- W3C SCXML 5.7")
class Test343 : W3CTestBase<Test343State, Test343Event>() {
    override fun createStateMachine() = Test343StateMachine(createEngine())
    override val expectedPassState: Test343State = Test343State.Pass
}
