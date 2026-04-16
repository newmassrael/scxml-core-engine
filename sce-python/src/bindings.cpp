// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/// SCE Python Bindings - pybind11 wrapper for ReadySCXMLEngine
/// Exposes the W3C SCXML 1.0 compliant C++ interpreter to Python

#include <pybind11/pybind11.h>
#include <pybind11/stl.h>

#include "ReadySCXMLEngine.h"

namespace py = pybind11;

namespace {

/// Python-friendly statistics snapshot.
/// Captures current state at query time (C++ Statistics only updates on transitions).
struct PyStatistics {
    int totalEvents = 0;
    int totalTransitions = 0;
    std::string currentState;
    bool isRunning = false;
};

/// Wrapper class that owns the C++ ReadySCXMLEngine via unique_ptr
/// and provides a Pythonic interface with properties and context manager.
class PyEngine {
public:
    explicit PyEngine(std::unique_ptr<SCE::ReadySCXMLEngine> engine)
        : engine_(std::move(engine)) {}

    PyEngine(const PyEngine &) = delete;
    PyEngine &operator=(const PyEngine &) = delete;
    PyEngine(PyEngine &&) = default;
    PyEngine &operator=(PyEngine &&) = default;

    // Factory methods - raise on failure instead of returning None
    static PyEngine fromFile(const std::string &path) {
        auto engine = SCE::ReadySCXMLEngine::fromFile(path);
        if (!engine) {
            const auto &detail = SCE::ReadySCXMLEngine::lastFactoryError();
            throw py::value_error("Failed to load SCXML file: " + path +
                                  (detail.empty() ? "" : " - " + detail));
        }
        return PyEngine(std::move(engine));
    }

    static PyEngine fromString(const std::string &content) {
        auto engine = SCE::ReadySCXMLEngine::fromString(content);
        if (!engine) {
            const auto &detail = SCE::ReadySCXMLEngine::lastFactoryError();
            throw py::value_error("Failed to parse SCXML content" +
                                  (detail.empty() ? std::string{} : " - " + detail));
        }
        return PyEngine(std::move(engine));
    }

    // Core operations — release GIL so C++ worker threads can run callbacks
    // without deadlocking against Python's GIL.
    bool start() {
        py::gil_scoped_release release;
        return engine_->start();
    }

    void stop() {
        py::gil_scoped_release release;
        engine_->stop();
    }

    bool sendEvent(const std::string &name, const std::string &data = "") {
        py::gil_scoped_release release;
        return engine_->sendEvent(name, data);
    }

    bool sendExternalEvent(const std::string &name, const std::string &data = "") {
        py::gil_scoped_release release;
        return engine_->sendExternalEvent(name, data);
    }

    // State queries — release GIL to avoid contention with processEventMutex_
    bool isRunning() const {
        py::gil_scoped_release release;
        return engine_->isRunning();
    }

    std::string getCurrentState() const {
        py::gil_scoped_release release;
        return engine_->getCurrentState();
    }

    bool isInState(const std::string &stateId) const {
        py::gil_scoped_release release;
        return engine_->isInState(stateId);
    }

    std::vector<std::string> getActiveStates() const {
        py::gil_scoped_release release;
        return engine_->getActiveStates();
    }

    // Variable access - single method with Python type dispatch
    bool setVariable(const std::string &name, const py::object &value) {
        // Order matters: bool before int (Python bool is subclass of int)
        // Extract the C++ value while GIL is held, then release for the C++ call.
        if (py::isinstance<py::bool_>(value)) {
            bool v = value.cast<bool>();
            py::gil_scoped_release release;
            return engine_->setVariable(name, v);
        }
        if (py::isinstance<py::int_>(value)) {
            int64_t v = value.cast<int64_t>();
            py::gil_scoped_release release;
            return engine_->setVariable(name, v);
        }
        if (py::isinstance<py::float_>(value)) {
            double v = value.cast<double>();
            py::gil_scoped_release release;
            return engine_->setVariable(name, v);
        }
        if (py::isinstance<py::str>(value)) {
            std::string v = value.cast<std::string>();
            py::gil_scoped_release release;
            return engine_->setVariable(name, v);
        }
        throw py::type_error(
            "set_variable: unsupported type '" +
            std::string(py::str(value.get_type().attr("__name__"))) +
            "', expected bool, int, float, or str");
    }

    std::string getVariable(const std::string &name) const {
        py::gil_scoped_release release;
        return engine_->getVariable(name);
    }

    // Error and statistics
    std::string getLastError() const {
        py::gil_scoped_release release;
        return engine_->getLastError();
    }

    PyStatistics getStatistics() const {
        py::gil_scoped_release release;
        auto s = engine_->getStatistics();
        PyStatistics ps;
        ps.totalEvents = s.totalEvents;
        ps.totalTransitions = s.totalTransitions;
        ps.currentState = engine_->getCurrentState();
        ps.isRunning = engine_->isRunning();
        return ps;
    }

    // Context manager — GIL released inside start()/stop() calls
    PyEngine &enter() {
        if (!start()) {
            throw py::value_error("Failed to start state machine: " + engine_->getLastError());
        }
        return *this;
    }

    void exit(const py::object & /*excType*/, const py::object & /*excVal*/, const py::object & /*excTb*/) {
        if (isRunning()) {
            stop();
        }
    }

    std::string repr() const {
        auto state = getCurrentState();
        auto running = isRunning();
        return "<sce.Engine state='" + (state.empty() ? "(none)" : state) +
               "' running=" + (running ? "True" : "False") + ">";
    }

private:
    std::unique_ptr<SCE::ReadySCXMLEngine> engine_;
};

}  // namespace

PYBIND11_MODULE(_sce, m) {
    m.doc() = "SCE - SCXML Core Engine Python bindings (W3C SCXML 1.0)";

    // Statistics struct
    py::class_<PyStatistics>(m, "Statistics",
        "State machine execution statistics")
        .def_readonly("total_events", &PyStatistics::totalEvents,
            "Total number of events processed")
        .def_readonly("total_transitions", &PyStatistics::totalTransitions,
            "Total number of state transitions")
        .def_readonly("current_state", &PyStatistics::currentState,
            "Current active state ID")
        .def_readonly("is_running", &PyStatistics::isRunning,
            "Whether the state machine is currently running")
        .def("__repr__", [](const PyStatistics &s) {
            return "<sce.Statistics events=" + std::to_string(s.totalEvents) +
                   " transitions=" + std::to_string(s.totalTransitions) +
                   " state='" + s.currentState + "'>";
        });

    // Engine class
    py::class_<PyEngine>(m, "Engine",
        "W3C SCXML 1.0 compliant state machine engine.\n\n"
        "Create from SCXML file or string, then start/stop and send events.\n"
        "Supports context manager protocol for automatic cleanup.")

        // Factory methods (static)
        .def_static("from_file", &PyEngine::fromFile,
            py::arg("path"),
            "Create engine from an SCXML file.\n\n"
            "Args:\n"
            "    path: Path to the SCXML file\n\n"
            "Raises:\n"
            "    ValueError: If file cannot be loaded or parsed")
        .def_static("from_string", &PyEngine::fromString,
            py::arg("content"),
            "Create engine from an SCXML string.\n\n"
            "Args:\n"
            "    content: SCXML document as string\n\n"
            "Raises:\n"
            "    ValueError: If content cannot be parsed")

        // Core operations
        .def("start", &PyEngine::start,
            "Start the state machine. Returns True if started successfully.")
        .def("stop", &PyEngine::stop,
            "Stop the state machine.")
        .def("send_event", &PyEngine::sendEvent,
            py::arg("name"), py::arg("data") = "",
            "Send an event to the state machine.\n\n"
            "Args:\n"
            "    name: Event name\n"
            "    data: Optional event data as JSON string\n\n"
            "Returns:\n"
            "    True if event was processed successfully")
        .def("send_external_event", &PyEngine::sendExternalEvent,
            py::arg("name"), py::arg("data") = "",
            "Send an external event to the state machine's external queue.\n\n"
            "Args:\n"
            "    name: Event name\n"
            "    data: Optional event data as JSON string\n\n"
            "Returns:\n"
            "    True if event was queued successfully")
        .def("is_in_state", &PyEngine::isInState,
            py::arg("state_id"),
            "Check if a specific state is currently active.")

        // Variable access
        .def("set_variable", &PyEngine::setVariable,
            py::arg("name"), py::arg("value"),
            "Set a variable in the data model.\n\n"
            "Accepts bool, int, float, or str values.\n"
            "Returns True if the variable was set successfully.\n\n"
            "Raises:\n"
            "    TypeError: If value type is not supported")
        .def("get_variable", &PyEngine::getVariable,
            py::arg("name"),
            "Get a variable from the data model as string.")

        // Read-only properties
        .def_property_readonly("running", &PyEngine::isRunning,
            "Whether the state machine is currently running.")
        .def_property_readonly("current_state", &PyEngine::getCurrentState,
            "Current active state ID (empty string if not started).")
        .def_property_readonly("active_states", &PyEngine::getActiveStates,
            "List of all currently active state IDs.")
        .def_property_readonly("last_error", &PyEngine::getLastError,
            "Last error message (empty string if no error).")
        .def_property_readonly("statistics", &PyEngine::getStatistics,
            "Execution statistics (events, transitions, state).")

        // Context manager
        .def("__enter__", &PyEngine::enter, py::return_value_policy::reference)
        .def("__exit__", &PyEngine::exit)

        // Representation
        .def("__repr__", &PyEngine::repr);
}
