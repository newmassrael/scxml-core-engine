// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// SCXMLContext.cpp
#include "SCXMLContext.h"

void SCE::SCXMLContext::setDatamodelType(const std::string &datamodelType) {
    datamodelType_ = datamodelType;
}

const std::string &SCE::SCXMLContext::getDatamodelType() const {
    return datamodelType_;
}

void SCE::SCXMLContext::setBinding(const std::string &binding) {
    binding_ = binding;
}

const std::string &SCE::SCXMLContext::getBinding() const {
    return binding_;
}

void SCE::SCXMLContext::addNamespace(const std::string &prefix, const std::string &uri) {
    namespaces_[prefix] = uri;
}

const std::string &SCE::SCXMLContext::getNamespaceURI(const std::string &prefix) const {
    auto it = namespaces_.find(prefix);
    if (it != namespaces_.end()) {
        return it->second;
    }
    return emptyString_;
}

void SCE::SCXMLContext::setAttribute(const std::string &name, const std::string &value) {
    attributes_[name] = value;
}

const std::string &SCE::SCXMLContext::getAttribute(const std::string &name) const {
    auto it = attributes_.find(name);
    if (it != attributes_.end()) {
        return it->second;
    }
    return emptyString_;
}
