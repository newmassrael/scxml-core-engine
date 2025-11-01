#pragma once

#include "common/Logger.h"
#include <functional>
#include <mutex>
#include <quickjs.h>
#include <string>
#include <type_traits>
#include <unordered_map>
#include <utility>

namespace RSM {

/**
 * @brief Helper for binding C++ classes to JavaScript using QuickJS
 *
 * Follows DOMBinding pattern with variadic templates for automatic signature support.
 * Supports methods with any number of arguments (0, 1, 2, ..., N).
 *
 * Thread-safe class registration using std::call_once.
 * Automatic type conversion between C++ and JavaScript types.
 *
 * Example usage:
 * @code
 * class Hardware {
 * public:
 *     double getTemperature() const { return temp_; }
 *     void setTemperature(double t) { temp_ = t; }
 *     void setRange(double min, double max) { min_ = min; max_ = max; }  // 2 args
 * private:
 *     double temp_ = 25.0;
 *     double min_ = 0.0;
 *     double max_ = 100.0;
 * };
 *
 * Hardware hw;
 * ClassBinder<Hardware> binder(ctx, "hardware", &hw);
 * binder.def("getTemperature", &Hardware::getTemperature)
 *       .def("setTemperature", &Hardware::setTemperature)
 *       .def("setRange", &Hardware::setRange);  // 2 args - auto supported!
 * JSValue jsObj = binder.finalize();
 * @endcode
 */
template <typename T> class ClassBinder {
public:
    ClassBinder(JSContext *ctx, const std::string &className, T *instance)
        : ctx_(ctx), className_(className), instance_(instance) {
        initializeClass();
        createJSObject();
    }

    /**
     * @brief Bind a non-void const method with any number of arguments
     *
     * Uses variadic templates to automatically support 0-N arguments.
     * Type conversion is automatic via fromJSValue/toJSValue helpers.
     *
     * Examples:
     * - `double getTemp() const` → 0 args
     * - `int calculate(double x) const` → 1 arg
     * - `string format(int a, double b) const` → 2 args
     */
    template <typename ReturnType, typename... Args>
    ClassBinder &def(const char *name, ReturnType (T::*method)(Args...) const) {
        return defImpl(name, method, std::make_index_sequence<sizeof...(Args)>{});
    }

    /**
     * @brief Bind a non-void non-const method with any number of arguments
     */
    template <typename ReturnType, typename... Args>
    ClassBinder &def(const char *name, ReturnType (T::*method)(Args...)) {
        return defImpl(name, method, std::make_index_sequence<sizeof...(Args)>{});
    }

    /**
     * @brief Bind a void const method with any number of arguments
     */
    template <typename... Args> ClassBinder &def(const char *name, void (T::*method)(Args...) const) {
        return defVoidImpl(name, method, std::make_index_sequence<sizeof...(Args)>{});
    }

    /**
     * @brief Bind a void non-const method with any number of arguments
     */
    template <typename... Args> ClassBinder &def(const char *name, void (T::*method)(Args...)) {
        return defVoidImpl(name, method, std::make_index_sequence<sizeof...(Args)>{});
    }

    JSValue finalize() {
        // Transfer ownership - return jsObject_ directly without duplication
        JSValue result = jsObject_;
        jsObject_ = JS_UNDEFINED;  // Clear internal reference to prevent double-free
        return result;
    }

private:
    JSContext *ctx_;
    std::string className_;
    T *instance_;
    JSValue jsObject_ = JS_UNDEFINED;

    static JSClassID &getClassID() {
        static JSClassID classID = 0;
        return classID;
    }

    static std::once_flag &getInitFlag() {
        static std::once_flag initFlag;
        return initFlag;
    }

    // Static method registry - one per C++ class type
    using MethodWrapper = std::function<JSValue(T *, JSContext *, int, JSValueConst *)>;

    static std::unordered_map<std::string, MethodWrapper> &getMethodRegistry() {
        static std::unordered_map<std::string, MethodWrapper> registry;
        return registry;
    }

    void initializeClass() {
        JSClassID &classID = getClassID();

        // Thread-safe class registration using std::call_once
        std::call_once(getInitFlag(), [this, &classID]() {
            JS_NewClassID(JS_GetRuntime(ctx_), &classID);
            JSClassDef classDef = {
                .class_name = className_.c_str(),
                .finalizer = nullptr,
                .gc_mark = nullptr,
                .call = nullptr,
                .exotic = nullptr,
            };
            JS_NewClass(JS_GetRuntime(ctx_), classID, &classDef);
        });
    }

    void createJSObject() {
        jsObject_ = JS_NewObjectClass(ctx_, getClassID());
        if (JS_IsException(jsObject_)) {
            LOG_ERROR("Failed to create JS object for class {}", className_);
            jsObject_ = JS_UNDEFINED;  // Explicit initialization on error
            return;
        }
        JS_SetOpaque(jsObject_, instance_);
    }

    /**
     * @brief Implementation for non-void const methods using index_sequence
     *
     * Uses fold expressions to unpack arguments: fromJSValue<Args>(ctx, argv[I])...
     * Each I corresponds to an argument index (0, 1, 2, ..., N-1)
     */
    template <typename ReturnType, typename... Args, std::size_t... I>
    ClassBinder &defImpl(const char *name, ReturnType (T::*method)(Args...) const, std::index_sequence<I...>) {
        std::string methodName(name);
        constexpr int argCount = sizeof...(Args);

        getMethodRegistry()[methodName] = [method](T *inst, JSContext *ctx, int argc, JSValueConst *argv) -> JSValue {
            if (argc < argCount) {
                return JS_ThrowTypeError(ctx, "Method requires %d argument(s)", argCount);
            }
            // Fold expression: convert each argument and invoke method
            ReturnType result = (inst->*method)(fromJSValue<Args>(ctx, argv[I])...);
            return toJSValue(ctx, result);
        };

        registerMethod(name, methodName, argCount);
        return *this;
    }

    /**
     * @brief Implementation for non-void non-const methods using index_sequence
     */
    template <typename ReturnType, typename... Args, std::size_t... I>
    ClassBinder &defImpl(const char *name, ReturnType (T::*method)(Args...), std::index_sequence<I...>) {
        std::string methodName(name);
        constexpr int argCount = sizeof...(Args);

        getMethodRegistry()[methodName] = [method](T *inst, JSContext *ctx, int argc, JSValueConst *argv) -> JSValue {
            if (argc < argCount) {
                return JS_ThrowTypeError(ctx, "Method requires %d argument(s)", argCount);
            }
            ReturnType result = (inst->*method)(fromJSValue<Args>(ctx, argv[I])...);
            return toJSValue(ctx, result);
        };

        registerMethod(name, methodName, argCount);
        return *this;
    }

    /**
     * @brief Implementation for void const methods using index_sequence
     */
    template <typename... Args, std::size_t... I>
    ClassBinder &defVoidImpl(const char *name, void (T::*method)(Args...) const, std::index_sequence<I...>) {
        std::string methodName(name);
        constexpr int argCount = sizeof...(Args);

        getMethodRegistry()[methodName] = [method](T *inst, JSContext *ctx, int argc, JSValueConst *argv) -> JSValue {
            if (argc < argCount) {
                return JS_ThrowTypeError(ctx, "Method requires %d argument(s)", argCount);
            }
            (inst->*method)(fromJSValue<Args>(ctx, argv[I])...);
            return JS_UNDEFINED;
        };

        registerMethod(name, methodName, argCount);
        return *this;
    }

    /**
     * @brief Implementation for void non-const methods using index_sequence
     */
    template <typename... Args, std::size_t... I>
    ClassBinder &defVoidImpl(const char *name, void (T::*method)(Args...), std::index_sequence<I...>) {
        std::string methodName(name);
        constexpr int argCount = sizeof...(Args);

        getMethodRegistry()[methodName] = [method](T *inst, JSContext *ctx, int argc, JSValueConst *argv) -> JSValue {
            if (argc < argCount) {
                return JS_ThrowTypeError(ctx, "Method requires %d argument(s)", argCount);
            }
            (inst->*method)(fromJSValue<Args>(ctx, argv[I])...);
            return JS_UNDEFINED;
        };

        registerMethod(name, methodName, argCount);
        return *this;
    }

    /**
     * @brief Common method registration logic
     *
     * Registers the method wrapper as a JavaScript function with func_data.
     * Note: JS_NewCFunctionData duplicates data values, so we must free the original.
     * Note: JS_SetPropertyStr takes ownership of funcObj, so we don't free it.
     */
    void registerMethod(const char *name, const std::string &methodName, int argCount) {
        JSValue methodNameVal = JS_NewString(ctx_, methodName.c_str());
        JSValue funcObj = JS_NewCFunctionData(ctx_, methodWrapper, argCount, 0, 1, &methodNameVal);
        JS_FreeValue(ctx_, methodNameVal);                  // JS_NewCFunctionData duplicates data values
        JS_SetPropertyStr(ctx_, jsObject_, name, funcObj);  // Takes ownership of funcObj
    }

    /**
     * @brief Universal method wrapper (follows DOMBinding pattern)
     *
     * Retrieves C++ instance from this_val, looks up method by name in registry,
     * and invokes the corresponding wrapper function.
     */
    static JSValue methodWrapper(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv, int magic,
                                 JSValue *func_data) {
        (void)magic;

        // Get C++ instance from this_val
        T *instance = static_cast<T *>(JS_GetOpaque(this_val, getClassID()));
        if (!instance) {
            return JS_ThrowTypeError(ctx, "Invalid object instance");
        }

        // Get method name from func_data (passed via JS_NewCFunctionData)
        if (!func_data || JS_IsUndefined(func_data[0])) {
            return JS_ThrowTypeError(ctx, "Method name not found in func_data");
        }

        const char *methodNameCStr = JS_ToCString(ctx, func_data[0]);
        if (!methodNameCStr) {
            return JS_ThrowTypeError(ctx, "Failed to get method name");
        }

        std::string methodName(methodNameCStr);
        JS_FreeCString(ctx, methodNameCStr);

        // Lookup wrapper in registry
        auto &registry = getMethodRegistry();
        auto it = registry.find(methodName);
        if (it == registry.end()) {
            std::string errorMsg = "Method not found: " + methodName;
            return JS_ThrowTypeError(ctx, "%s", errorMsg.c_str());
        }

        // Call wrapper
        return it->second(instance, ctx, argc, argv);
    }

    /**
     * @brief Convert C++ value to JavaScript value
     *
     * Supports: double, float, int, long, int32_t, bool, std::string, const char*
     */
    template <typename U> static JSValue toJSValue(JSContext *ctx, const U &value) {
        if constexpr (std::is_same_v<U, double> || std::is_same_v<U, float>) {
            return JS_NewFloat64(ctx, static_cast<double>(value));
        } else if constexpr (std::is_same_v<U, int> || std::is_same_v<U, long> || std::is_same_v<U, int32_t>) {
            return JS_NewInt32(ctx, static_cast<int32_t>(value));
        } else if constexpr (std::is_same_v<U, bool>) {
            return JS_NewBool(ctx, value);
        } else if constexpr (std::is_same_v<U, std::string>) {
            return JS_NewString(ctx, value.c_str());
        } else if constexpr (std::is_same_v<U, const char *>) {
            return JS_NewString(ctx, value);
        } else {
            static_assert(sizeof(U) == 0, "Unsupported return type");
        }
    }

    /**
     * @brief Convert JavaScript value to C++ value
     *
     * Uses std::decay_t to handle const/reference qualifiers automatically.
     * Supports: double, float, int, int32_t, bool, std::string
     */
    template <typename U> static std::decay_t<U> fromJSValue(JSContext *ctx, JSValueConst val) {
        using CleanType = std::decay_t<U>;

        if constexpr (std::is_same_v<CleanType, double> || std::is_same_v<CleanType, float>) {
            double result = 0.0;
            JS_ToFloat64(ctx, &result, val);
            return static_cast<CleanType>(result);
        } else if constexpr (std::is_same_v<CleanType, int> || std::is_same_v<CleanType, int32_t>) {
            int32_t result = 0;
            JS_ToInt32(ctx, &result, val);
            return static_cast<CleanType>(result);
        } else if constexpr (std::is_same_v<CleanType, bool>) {
            return JS_ToBool(ctx, val);
        } else if constexpr (std::is_same_v<CleanType, std::string>) {
            const char *str = JS_ToCString(ctx, val);
            if (!str) {
                return std::string("");
            }
            std::string result(str);
            JS_FreeCString(ctx, str);
            return result;
        } else {
            static_assert(sizeof(U) == 0, "Unsupported argument type");
        }
    }
};

}  // namespace RSM
