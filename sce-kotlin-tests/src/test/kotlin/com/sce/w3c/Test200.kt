// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test200.scxml:1
package com.sce.w3c

import com.sce.generated.test200.Test200Event
import com.sce.generated.test200.Test200State
import com.sce.generated.test200.Test200StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: SCXML Processors MUST support the type http://www.w3.org/TR/scxml/#SCXMLEventProcessor
@DisplayName("Test 200 -- W3C SCXML 6.2")
class Test200 : W3CTestBase<Test200State, Test200Event>() {
    override fun createStateMachine() = Test200StateMachine()
    override val expectedPassState: Test200State = Test200State.Pass
}
