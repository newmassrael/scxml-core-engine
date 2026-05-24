// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test152.scxml:1
package com.sce.w3c

import com.sce.generated.test152.Test152Event
import com.sce.generated.test152.Test152State
import com.sce.generated.test152.Test152StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: In the foreach element, if 'array' does not evaluate to a legal iterable collection, or if 'item' does not specify a legal variable name, the SCXML processor MUST terminate execution of the foreach element and the block that contains it, and place the error error.execution on the internal event queue.
@DisplayName("Test 152 -- W3C SCXML 4.6")
class Test152 : W3CTestBase<Test152State, Test152Event>() {
    override fun createStateMachine() = Test152StateMachine(createEngine())
    override val expectedPassState: Test152State = Test152State.Pass
}
