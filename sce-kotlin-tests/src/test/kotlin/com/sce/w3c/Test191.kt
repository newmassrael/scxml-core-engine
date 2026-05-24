// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test191.scxml:1
package com.sce.w3c

import com.sce.generated.test191.Test191Event
import com.sce.generated.test191.Test191State
import com.sce.generated.test191.Test191StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: [When using the scxml event i/o processor] If the target is the special term '#_parent', the Processor MUST add the event to the external event queue of the SCXML session that invoked the sending session, if there is one.
@DisplayName("Test 191 -- W3C SCXML C.1")
class Test191 : W3CTestBase<Test191State, Test191Event>() {
    override fun createStateMachine() = Test191StateMachine()
    override val expectedPassState: Test191State = Test191State.Pass
    override val timeoutMs: Long = 5000L
}
