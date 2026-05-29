// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5af0768adc0cd444b401fc40536c0de87cadf9b1f8be7299536f4fc9ed22e337
// generated-at: 1780020098
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test322.scxml:1
package com.sce.w3c

import com.sce.generated.test322.Test322Event
import com.sce.generated.test322.Test322State
import com.sce.generated.test322.Test322StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST keep the _sessionid variable bound to the system-generated id until the session terminates.
@DisplayName("Test 322 -- W3C SCXML 5.10")
class Test322 : W3CTestBase<Test322State, Test322Event>() {
    override fun createStateMachine() = Test322StateMachine(createEngine())
    override val expectedPassState: Test322State = Test322State.Pass
}
