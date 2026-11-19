// SCE-GENERATED — DO NOT EDIT
// source-hash: e8f320f75dfeb19d8a1c86b952355e53aa00519f4b2e9108eb558cc94a73e2ba
// source: error_reason.scxml
// SCE-MAP: error_reason.scxml:60 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package error_reason

import "github.com/newmassrael/sce-forge-runtime/optional"

type ErrorReason uint8

const (
	ErrorReasonExpressionFailed ErrorReason = 1
	ErrorReasonLocationInvalid ErrorReason = 2
	ErrorReasonAssignmentFailed ErrorReason = 3
	ErrorReasonScriptFailed ErrorReason = 4
	ErrorReasonDataInitFailed ErrorReason = 5
	ErrorReasonForeachInvalid ErrorReason = 6
	ErrorReasonSendTypeUnsupported ErrorReason = 7
	ErrorReasonSendTargetInvalid ErrorReason = 8
	ErrorReasonSendDelayInvalid ErrorReason = 9
	ErrorReasonParamUnfit ErrorReason = 10
	ErrorReasonProcessorUnregistered ErrorReason = 11
	ErrorReasonInvokeTypeUnsupported ErrorReason = 12
	ErrorReasonInvokeSrcNotFound ErrorReason = 13
	ErrorReasonInvokeChildInitFailed ErrorReason = 14
	ErrorReasonReservedParamConflict ErrorReason = 15
	ErrorReasonSessionTransportUnavailable ErrorReason = 16
	ErrorReasonPayloadRefused ErrorReason = 17
	ErrorReasonCapacityExceeded ErrorReason = 18
	ErrorReasonTargetUnreachable ErrorReason = 64
	ErrorReasonHttpTargetInvalid ErrorReason = 65
	ErrorReasonTransportUnavailable ErrorReason = 66
	ErrorReasonSendFailed ErrorReason = 67
	ErrorReasonDeliveryExhausted ErrorReason = 68
	ErrorReasonEnvelopeCorrupt ErrorReason = 69
	ErrorReasonInvokeChildLost ErrorReason = 70
	ErrorReasonParallelBarrierTimeout ErrorReason = 71
	ErrorReasonDedupWindowOverflow ErrorReason = 72
	ErrorReasonPeerPartitioned ErrorReason = 73
	ErrorReasonBackpressureDrop ErrorReason = 74
	ErrorReasonUnauthorized ErrorReason = 75
	ErrorReasonMissingSequence ErrorReason = 76
	ErrorReasonOrderingGap ErrorReason = 77
	ErrorReasonRegionPartitioned ErrorReason = 78
	ErrorReasonRpcReplyFromUndeclaredPeer ErrorReason = 79
	ErrorReasonOutboundStaleDrop ErrorReason = 80
	ErrorReasonNotificationDeadlineMissed ErrorReason = 81
	ErrorReasonRestored ErrorReason = 82
	ErrorReasonCausalViewMismatch ErrorReason = 83
)

// ToUnderlying returns the carrier value this holds.
func (v ErrorReason) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v ErrorReason) DeclaredName() optional.Of[string] {
	switch v {
	case ErrorReasonExpressionFailed:
		return optional.Some("expressionFailed")
	case ErrorReasonLocationInvalid:
		return optional.Some("locationInvalid")
	case ErrorReasonAssignmentFailed:
		return optional.Some("assignmentFailed")
	case ErrorReasonScriptFailed:
		return optional.Some("scriptFailed")
	case ErrorReasonDataInitFailed:
		return optional.Some("dataInitFailed")
	case ErrorReasonForeachInvalid:
		return optional.Some("foreachInvalid")
	case ErrorReasonSendTypeUnsupported:
		return optional.Some("sendTypeUnsupported")
	case ErrorReasonSendTargetInvalid:
		return optional.Some("sendTargetInvalid")
	case ErrorReasonSendDelayInvalid:
		return optional.Some("sendDelayInvalid")
	case ErrorReasonParamUnfit:
		return optional.Some("paramUnfit")
	case ErrorReasonProcessorUnregistered:
		return optional.Some("processorUnregistered")
	case ErrorReasonInvokeTypeUnsupported:
		return optional.Some("invokeTypeUnsupported")
	case ErrorReasonInvokeSrcNotFound:
		return optional.Some("invokeSrcNotFound")
	case ErrorReasonInvokeChildInitFailed:
		return optional.Some("invokeChildInitFailed")
	case ErrorReasonReservedParamConflict:
		return optional.Some("reservedParamConflict")
	case ErrorReasonSessionTransportUnavailable:
		return optional.Some("sessionTransportUnavailable")
	case ErrorReasonPayloadRefused:
		return optional.Some("payloadRefused")
	case ErrorReasonCapacityExceeded:
		return optional.Some("capacityExceeded")
	case ErrorReasonTargetUnreachable:
		return optional.Some("targetUnreachable")
	case ErrorReasonHttpTargetInvalid:
		return optional.Some("httpTargetInvalid")
	case ErrorReasonTransportUnavailable:
		return optional.Some("transportUnavailable")
	case ErrorReasonSendFailed:
		return optional.Some("sendFailed")
	case ErrorReasonDeliveryExhausted:
		return optional.Some("deliveryExhausted")
	case ErrorReasonEnvelopeCorrupt:
		return optional.Some("envelopeCorrupt")
	case ErrorReasonInvokeChildLost:
		return optional.Some("invokeChildLost")
	case ErrorReasonParallelBarrierTimeout:
		return optional.Some("parallelBarrierTimeout")
	case ErrorReasonDedupWindowOverflow:
		return optional.Some("dedupWindowOverflow")
	case ErrorReasonPeerPartitioned:
		return optional.Some("peerPartitioned")
	case ErrorReasonBackpressureDrop:
		return optional.Some("backpressureDrop")
	case ErrorReasonUnauthorized:
		return optional.Some("unauthorized")
	case ErrorReasonMissingSequence:
		return optional.Some("missingSequence")
	case ErrorReasonOrderingGap:
		return optional.Some("orderingGap")
	case ErrorReasonRegionPartitioned:
		return optional.Some("regionPartitioned")
	case ErrorReasonRpcReplyFromUndeclaredPeer:
		return optional.Some("rpcReplyFromUndeclaredPeer")
	case ErrorReasonOutboundStaleDrop:
		return optional.Some("outboundStaleDrop")
	case ErrorReasonNotificationDeadlineMissed:
		return optional.Some("notificationDeadlineMissed")
	case ErrorReasonRestored:
		return optional.Some("restored")
	case ErrorReasonCausalViewMismatch:
		return optional.Some("causalViewMismatch")
	}
	return optional.None[string]()
}

// ErrorReasonFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func ErrorReasonFromUnderlying(raw uint8) optional.Of[ErrorReason] {
	switch raw {
	case 1:
		return optional.Some(ErrorReasonExpressionFailed)
	case 2:
		return optional.Some(ErrorReasonLocationInvalid)
	case 3:
		return optional.Some(ErrorReasonAssignmentFailed)
	case 4:
		return optional.Some(ErrorReasonScriptFailed)
	case 5:
		return optional.Some(ErrorReasonDataInitFailed)
	case 6:
		return optional.Some(ErrorReasonForeachInvalid)
	case 7:
		return optional.Some(ErrorReasonSendTypeUnsupported)
	case 8:
		return optional.Some(ErrorReasonSendTargetInvalid)
	case 9:
		return optional.Some(ErrorReasonSendDelayInvalid)
	case 10:
		return optional.Some(ErrorReasonParamUnfit)
	case 11:
		return optional.Some(ErrorReasonProcessorUnregistered)
	case 12:
		return optional.Some(ErrorReasonInvokeTypeUnsupported)
	case 13:
		return optional.Some(ErrorReasonInvokeSrcNotFound)
	case 14:
		return optional.Some(ErrorReasonInvokeChildInitFailed)
	case 15:
		return optional.Some(ErrorReasonReservedParamConflict)
	case 16:
		return optional.Some(ErrorReasonSessionTransportUnavailable)
	case 17:
		return optional.Some(ErrorReasonPayloadRefused)
	case 18:
		return optional.Some(ErrorReasonCapacityExceeded)
	case 64:
		return optional.Some(ErrorReasonTargetUnreachable)
	case 65:
		return optional.Some(ErrorReasonHttpTargetInvalid)
	case 66:
		return optional.Some(ErrorReasonTransportUnavailable)
	case 67:
		return optional.Some(ErrorReasonSendFailed)
	case 68:
		return optional.Some(ErrorReasonDeliveryExhausted)
	case 69:
		return optional.Some(ErrorReasonEnvelopeCorrupt)
	case 70:
		return optional.Some(ErrorReasonInvokeChildLost)
	case 71:
		return optional.Some(ErrorReasonParallelBarrierTimeout)
	case 72:
		return optional.Some(ErrorReasonDedupWindowOverflow)
	case 73:
		return optional.Some(ErrorReasonPeerPartitioned)
	case 74:
		return optional.Some(ErrorReasonBackpressureDrop)
	case 75:
		return optional.Some(ErrorReasonUnauthorized)
	case 76:
		return optional.Some(ErrorReasonMissingSequence)
	case 77:
		return optional.Some(ErrorReasonOrderingGap)
	case 78:
		return optional.Some(ErrorReasonRegionPartitioned)
	case 79:
		return optional.Some(ErrorReasonRpcReplyFromUndeclaredPeer)
	case 80:
		return optional.Some(ErrorReasonOutboundStaleDrop)
	case 81:
		return optional.Some(ErrorReasonNotificationDeadlineMissed)
	case 82:
		return optional.Some(ErrorReasonRestored)
	case 83:
		return optional.Some(ErrorReasonCausalViewMismatch)
	}
	return optional.None[ErrorReason]()
}
