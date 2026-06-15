// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 030a39123c8149accb30146fc4a4999b6e8826a330653d219a562116c552e0d8
// generated-at: 1781483328
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test521.scxml:1
package com.sce.w3c

import com.sce.generated.test521.Test521Event
import com.sce.generated.test521.Test521State
import com.sce.generated.test521.Test521StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: f the Processor cannot dispatch the event, it MUST place the error error.communication on the internal event queue of the session that attempted to send the event.
@DisplayName("Test 521 -- W3C SCXML 6.2")
class Test521 : W3CTestBase<Test521State, Test521Event>() {
    override fun createStateMachine() = Test521StateMachine(createEngine())
    override val expectedPassState: Test521State = Test521State.Pass
}
