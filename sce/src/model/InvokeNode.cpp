// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// InvokeNode.cpp
#include "InvokeNode.h"
#include "core/LogMacros.h"

SCE::InvokeNode::InvokeNode(const std::string &id) : id_(id), autoForward_(false) {
    SCE_LOG_DEBUG("Creating invoke node: {}", id);
}

SCE::InvokeNode::~InvokeNode() {
    SCE_LOG_DEBUG("Destroying invoke node: {}", id_);
}

const std::string &SCE::InvokeNode::getId() const {
    return id_;
}

void SCE::InvokeNode::setId(const std::string &id) {
    SCE_LOG_DEBUG("Setting ID for invoke: {} -> {}", id_, id);
    id_ = id;
}

const std::string &SCE::InvokeNode::getType() const {
    return type_;
}

const std::string &SCE::InvokeNode::getSrc() const {
    return src_;
}

bool SCE::InvokeNode::isAutoForward() const {
    return autoForward_;
}

void SCE::InvokeNode::setType(const std::string &type) {
    SCE_LOG_DEBUG("Setting type for {}: {}", id_, type);
    type_ = type;
}

void SCE::InvokeNode::setSrc(const std::string &src) {
    SCE_LOG_DEBUG("Setting src for {}: {}", id_, src);
    src_ = src;
}

void SCE::InvokeNode::setAutoForward(bool autoForward) {
    SCE_LOG_DEBUG("Setting autoForward for {}: {}", id_, (autoForward ? "true" : "false"));
    autoForward_ = autoForward;
}

void SCE::InvokeNode::setIdLocation(const std::string &idLocation) {
    SCE_LOG_DEBUG("Setting idLocation for {}: {}", id_, idLocation);
    idLocation_ = idLocation;
}

void SCE::InvokeNode::setNamelist(const std::string &namelist) {
    SCE_LOG_DEBUG("Setting namelist for {}: {}", id_, namelist);
    namelist_ = namelist;
}

void SCE::InvokeNode::addParam(const std::string &name, const std::string &expr, const std::string &location) {
    SCE_LOG_DEBUG("Adding param to {}: name={}", id_, name);
    params_.push_back(std::make_tuple(name, expr, location));
}

void SCE::InvokeNode::setContent(const std::string &content) {
    SCE_LOG_DEBUG("Setting content for {}", id_);
    content_ = content;
}

void SCE::InvokeNode::setFinalize(const std::string &finalizeContent) {
    SCE_LOG_DEBUG("Setting finalize for {}", id_);
    finalize_ = finalizeContent;
}

const std::string &SCE::InvokeNode::getIdLocation() const {
    return idLocation_;
}

const std::string &SCE::InvokeNode::getNamelist() const {
    return namelist_;
}

const std::string &SCE::InvokeNode::getContent() const {
    return content_;
}

const std::string &SCE::InvokeNode::getFinalize() const {
    return finalize_;
}

const std::vector<std::tuple<std::string, std::string, std::string>> &SCE::InvokeNode::getParams() const {
    return params_;
}

// W3C SCXML 1.0: typeexpr attribute support for dynamic type evaluation
void SCE::InvokeNode::setTypeExpr(const std::string &typeExpr) {
    typeExpr_ = typeExpr;
    SCE_LOG_DEBUG("InvokeNode: Set typeexpr to '{}'", typeExpr);
}

const std::string &SCE::InvokeNode::getTypeExpr() const {
    return typeExpr_;
}

// W3C SCXML 1.0: srcexpr attribute support for dynamic source evaluation
void SCE::InvokeNode::setSrcExpr(const std::string &srcExpr) {
    srcExpr_ = srcExpr;
    SCE_LOG_DEBUG("InvokeNode: Set srcexpr to '{}'", srcExpr);
}

const std::string &SCE::InvokeNode::getSrcExpr() const {
    return srcExpr_;
}

// W3C SCXML test 530: content expr attribute support for dynamic content evaluation
void SCE::InvokeNode::setContentExpr(const std::string &contentExpr) {
    contentExpr_ = contentExpr;
    SCE_LOG_DEBUG("InvokeNode: Set content expr to '{}'", contentExpr);
}

const std::string &SCE::InvokeNode::getContentExpr() const {
    return contentExpr_;
}

// W3C SCXML 6.4: State ID for invoke ID generation in "stateid.platformid" format (test 224)
void SCE::InvokeNode::setStateId(const std::string &stateId) {
    stateId_ = stateId;
    SCE_LOG_DEBUG("InvokeNode: Set state ID to '{}'", stateId);
}

const std::string &SCE::InvokeNode::getStateId() const {
    return stateId_;
}
