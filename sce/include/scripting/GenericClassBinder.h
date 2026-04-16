// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IScriptEngine.h"
#include "SCXMLTypes.h"
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

namespace SCE {

/**
 * @brief Engine-agnostic C++ class binder using ScriptValue type system
 *
 * Provides the same fluent .def() API as ClassBinder<T> but works with any
 * IScriptEngine implementation (QuickJS, Lua, etc.) through the bindNativeObject interface.
 *
 * Type conversions go through ScriptValue (engine-agnostic variant type) rather than
 * engine-specific types like JSValue or lua_State values.
 *
 * Example usage:
 * @code
 * Hardware hw;
 * GenericClassBinder<Hardware> binder("hardware", &hw);
 * binder.def("getTemperature", &Hardware::getTemperature)
 *       .def("setTemperature", &Hardware::setTemperature)
 *       .def("setRange", &Hardware::setRange);
 *
 * scriptEngine.bindNativeObject(sessionId, "hardware", binder.getMethods());
 * @endcode
 */
template <typename T>
class GenericClassBinder {
public:
    using NativeMethod = IScriptEngine::NativeMethod;

    GenericClassBinder(const std::string &className, T *instance) : className_(className), instance_(instance) {}

    /**
     * @brief Bind a non-void const method with any number of arguments
     */
    template <typename ReturnType, typename... Args>
    GenericClassBinder &def(const char *name, ReturnType (T::*method)(Args...) const) {
        addNonVoidMethod(name, method, std::index_sequence_for<Args...>{});
        return *this;
    }

    /**
     * @brief Bind a non-void non-const method with any number of arguments
     */
    template <typename ReturnType, typename... Args>
    GenericClassBinder &def(const char *name, ReturnType (T::*method)(Args...)) {
        addNonVoidMethod(name, method, std::index_sequence_for<Args...>{});
        return *this;
    }

    /**
     * @brief Bind a void const method with any number of arguments
     */
    template <typename... Args>
    GenericClassBinder &def(const char *name, void (T::*method)(Args...) const) {
        addVoidMethod(name, method, std::index_sequence_for<Args...>{});
        return *this;
    }

    /**
     * @brief Bind a void non-const method with any number of arguments
     */
    template <typename... Args>
    GenericClassBinder &def(const char *name, void (T::*method)(Args...)) {
        addVoidMethod(name, method, std::index_sequence_for<Args...>{});
        return *this;
    }

    const std::string &getClassName() const { return className_; }

    const std::vector<std::pair<std::string, NativeMethod>> &getMethods() const { return methods_; }

private:
    std::string className_;
    T *instance_;
    std::vector<std::pair<std::string, NativeMethod>> methods_;

    // === ScriptValue → C++ type conversion ===

    template <typename U>
    static std::decay_t<U> fromScriptValue(const ScriptValue &val) {
        using CleanType = std::decay_t<U>;

        if constexpr (std::is_same_v<CleanType, bool>) {
            if (auto *v = std::get_if<bool>(&val)) return *v;
            if (auto *v = std::get_if<int64_t>(&val)) return *v != 0;
            if (auto *v = std::get_if<double>(&val)) return *v != 0.0;
            return false;
        } else if constexpr (std::is_same_v<CleanType, double> || std::is_same_v<CleanType, float>) {
            if (auto *v = std::get_if<double>(&val)) return static_cast<CleanType>(*v);
            if (auto *v = std::get_if<int64_t>(&val)) return static_cast<CleanType>(*v);
            return CleanType{};
        } else if constexpr (std::is_integral_v<CleanType>) {
            if (auto *v = std::get_if<int64_t>(&val)) return static_cast<CleanType>(*v);
            if (auto *v = std::get_if<double>(&val)) return static_cast<CleanType>(*v);
            if (auto *v = std::get_if<bool>(&val)) return static_cast<CleanType>(*v);
            return CleanType{};
        } else if constexpr (std::is_same_v<CleanType, std::string>) {
            if (auto *v = std::get_if<std::string>(&val)) return *v;
            return std::string{};
        } else {
            static_assert(sizeof(CleanType) == 0, "Unsupported argument type for GenericClassBinder");
        }
    }

    // === C++ type → ScriptValue conversion ===

    template <typename U>
    static ScriptValue toScriptValue(const U &val) {
        using CleanType = std::decay_t<U>;

        if constexpr (std::is_same_v<CleanType, bool>) {
            return ScriptValue(val);
        } else if constexpr (std::is_same_v<CleanType, double> || std::is_same_v<CleanType, float>) {
            return ScriptValue(static_cast<double>(val));
        } else if constexpr (std::is_integral_v<CleanType>) {
            return ScriptValue(static_cast<int64_t>(val));
        } else if constexpr (std::is_same_v<CleanType, std::string>) {
            return ScriptValue(val);
        } else if constexpr (std::is_same_v<CleanType, const char *>) {
            return ScriptValue(std::string(val));
        } else {
            static_assert(sizeof(CleanType) == 0, "Unsupported return type for GenericClassBinder");
        }
    }

    // === Non-void method wrapper (const + non-const) ===

    template <typename ReturnType, typename... Args, std::size_t... I>
    void addNonVoidMethod(const char *name, ReturnType (T::*method)(Args...) const, std::index_sequence<I...>) {
        T *inst = instance_;
        NativeMethod wrapper = [inst, method](const std::vector<ScriptValue> &args) -> ScriptValue {
            if (args.size() < sizeof...(Args)) {
                return ScriptValue(ScriptUndefined{});
            }
            return toScriptValue<ReturnType>((inst->*method)(fromScriptValue<Args>(args[I])...));
        };
        methods_.emplace_back(name, std::move(wrapper));
    }

    template <typename ReturnType, typename... Args, std::size_t... I>
    void addNonVoidMethod(const char *name, ReturnType (T::*method)(Args...), std::index_sequence<I...>) {
        T *inst = instance_;
        NativeMethod wrapper = [inst, method](const std::vector<ScriptValue> &args) -> ScriptValue {
            if (args.size() < sizeof...(Args)) {
                return ScriptValue(ScriptUndefined{});
            }
            return toScriptValue<ReturnType>((inst->*method)(fromScriptValue<Args>(args[I])...));
        };
        methods_.emplace_back(name, std::move(wrapper));
    }

    // === Void method wrapper (const + non-const) ===

    template <typename... Args, std::size_t... I>
    void addVoidMethod(const char *name, void (T::*method)(Args...) const, std::index_sequence<I...>) {
        T *inst = instance_;
        NativeMethod wrapper = [inst, method](const std::vector<ScriptValue> &args) -> ScriptValue {
            if (args.size() < sizeof...(Args)) {
                return ScriptValue(ScriptUndefined{});
            }
            (inst->*method)(fromScriptValue<Args>(args[I])...);
            return ScriptValue(ScriptUndefined{});
        };
        methods_.emplace_back(name, std::move(wrapper));
    }

    template <typename... Args, std::size_t... I>
    void addVoidMethod(const char *name, void (T::*method)(Args...), std::index_sequence<I...>) {
        T *inst = instance_;
        NativeMethod wrapper = [inst, method](const std::vector<ScriptValue> &args) -> ScriptValue {
            if (args.size() < sizeof...(Args)) {
                return ScriptValue(ScriptUndefined{});
            }
            (inst->*method)(fromScriptValue<Args>(args[I])...);
            return ScriptValue(ScriptUndefined{});
        };
        methods_.emplace_back(name, std::move(wrapper));
    }
};

}  // namespace SCE
