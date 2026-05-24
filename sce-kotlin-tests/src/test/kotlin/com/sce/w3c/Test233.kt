// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test233.scxml:1
package com.sce.w3c

import com.sce.generated.test233.Test233Event
import com.sce.generated.test233.Test233State
import com.sce.generated.test233.Test233StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If there is a finalize handler in the instance of invoke that created the service that generated the event, the SCXML Processor MUST execute the code in that finalize handler right before it removes the event from the event queue for processing.
@DisplayName("Test 233 -- W3C SCXML 6.4")
class Test233 : W3CTestBase<Test233State, Test233Event>() {
    override fun createStateMachine() = Test233StateMachine(createEngine())
    override val expectedPassState: Test233State = Test233State.Pass
}
