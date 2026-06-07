// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 66bc1c3694f90e60100c842d2a53cd8c05682260c1809ba387d157940d7d6e1d
// generated-at: 1780836426
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test155.scxml:1
package com.sce.w3c

import com.sce.generated.test155.Test155Event
import com.sce.generated.test155.Test155State
import com.sce.generated.test155.Test155StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: when evaluating foreach, for each item, after making the assignment, the SCXML processor MUST evaluate its child executable content. It MUST then proceed to the next item in iteration order.
@DisplayName("Test 155 -- W3C SCXML 4.6")
class Test155 : W3CTestBase<Test155State, Test155Event>() {
    override fun createStateMachine() = Test155StateMachine(createEngine())
    override val expectedPassState: Test155State = Test155State.Pass
}
