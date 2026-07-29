// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa58405544015ba4d1b8207b13e783fe4f4b991c1d05b4cc1602d85ec7348310
// generated-at: 1785367096
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
