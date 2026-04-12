// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include <iostream>

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
