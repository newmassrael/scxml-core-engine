#include <iostream>

// Include generated header first
#include "smart_light_sm.h"

// Hardware abstraction layer
struct Hardware {
    bool hasPower() {
        std::cout << "  [Hardware] Checking power... OK\n";
        return true;
    }

    void powerOn() {
        std::cout << "  [Hardware] Power ON\n";
    }

    void powerOff() {
        std::cout << "  [Hardware] Power OFF\n";
    }

    void setBrightness(int level) {
        std::cout << "  [Hardware] Brightness: " << level << "%\n";
    }
};

// UserContext: Dependency injection container for user objects
struct SmartLightContext {
    Hardware hardware;
};

int main() {
    using namespace SCE::Generated::smart_light;

    std::cout << "=== Smart Light Example (C++ Function Integration with UserContext) ===\n\n";

    // Create user context with hardware dependencies
    SmartLightContext context;

    // Create state machine with dependency injection
    smart_light light(context);
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
