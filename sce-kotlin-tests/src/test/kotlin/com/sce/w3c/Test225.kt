// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test225.scxml:1
package com.sce.w3c

import com.sce.generated.test225.Test225Event
import com.sce.generated.test225.Test225State
import com.sce.generated.test225.Test225StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: n the automatically generated invoke identifier, platformid MUST be unique within the current session
@DisplayName("Test 225 -- W3C SCXML 6.4")
class Test225 : W3CTestBase<Test225State, Test225Event>() {
    override fun createStateMachine() = Test225StateMachine(createEngine())
    override val expectedPassState: Test225State = Test225State.Pass
}
