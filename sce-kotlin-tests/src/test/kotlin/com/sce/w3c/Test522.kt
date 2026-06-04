// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test522.scxml:1
package com.sce.w3c

import com.sce.generated.test522.Test522Event
import com.sce.generated.test522.Test522State
import com.sce.generated.test522.Test522StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: SCXML Processors that support the BasicHTTP Event I/O Processor MUST maintain a 'basichttp' entry in the _ioprocessors system variable. The Processor MUST maintain a in 'location' field inside this entry whose value holds an address that external entities can use to communicate with this SCXML session using the Basic HTTP Event I/O Processor.
@DisplayName("Test 522 -- W3C SCXML C.2")
class Test522 : W3CHttpTestBase<Test522State, Test522Event>() {
    override fun createStateMachine() = Test522StateMachine()
    override val expectedPassState: Test522State = Test522State.Pass
}
