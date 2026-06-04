// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test224.scxml:1
package com.sce.w3c

import com.sce.generated.test224.Test224Event
import com.sce.generated.test224.Test224State
import com.sce.generated.test224.Test224StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the platform generates an identifier for 'idlocation', the identifier MUST have the form stateid.platformid, where stateid is the id of the state containing this element and platformid is automatically generated.
@DisplayName("Test 224 -- W3C SCXML 6.4")
class Test224 : W3CTestBase<Test224State, Test224Event>() {
    override fun createStateMachine() = Test224StateMachine(createEngine())
    override val expectedPassState: Test224State = Test224State.Pass
    override val timeoutMs: Long = 5000L
}
