#include <iostream>
#include "smart_light_sm.h"

int main() {
    using namespace SCE::Generated::smart_light;

    std::cout << "=== Smart Light Example (Named Context) ===\n\n";

    // Named Context: pass hardware directly (no wrapper struct needed)
    Hardware hardware;

    // Create state machine with Named Context dependency injection
    smart_light light(hardware);
    light.initialize();
    std::cout << "State machine initialized\n\n";

    // Test 1: OFF → ON
    std::cout << "Test 1: Turning light on...\n";
    light.raiseExternal(Event::Switch_on);
    light.step();
    std::cout << "\n";

    // Test 2: ON → OFF
    std::cout << "Test 2: Turning light off...\n";
    light.raiseExternal(Event::Switch_off);
    light.step();
    std::cout << "\n";

    // Test 3: OFF → ON again
    std::cout << "Test 3: Turning light on again...\n";
    light.raiseExternal(Event::Switch_on);
    light.step();
    std::cout << "\n";

    std::cout << "=== All tests passed! ===\n";
    return 0;
}
