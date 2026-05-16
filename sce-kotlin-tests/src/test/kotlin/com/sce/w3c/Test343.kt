// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932425
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
