#include "..\inc\filter.h"

static VOID MapOperStatus(_In_ NDIS_MEDIA_CONNECT_STATE ConnectState, _Out_ PULONG OperStatus);

NDIS_STATUS
FilterAttach(
    _In_ NDIS_HANDLE NdisFilterHandle,
    _In_ NDIS_HANDLE FilterDriverContext,
    _In_ PNDIS_FILTER_ATTACH_PARAMETERS AttachParameters
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext;
    NDIS_FILTER_ATTRIBUTES filterAttributes;
    NDIS_STATUS status = NDIS_STATUS_SUCCESS;

    UNREFERENCED_PARAMETER(FilterDriverContext);

    adapterContext = (PNETOBS_ADAPTER_CONTEXT)ExAllocatePool2(
        POOL_FLAG_NON_PAGED, sizeof(NETOBS_ADAPTER_CONTEXT), NETOBS_POOL_TAG);
    if (adapterContext == NULL) {
        return NDIS_STATUS_RESOURCES;
    }
    RtlZeroMemory(adapterContext, sizeof(NETOBS_ADAPTER_CONTEXT));

    adapterContext->FilterHandle = NdisFilterHandle;
    adapterContext->IfIndex = AttachParameters->BaseMiniportIfIndex;
    NdisAllocateSpinLock(&adapterContext->Lock);

    RtlZeroMemory(&filterAttributes, sizeof(filterAttributes));
    filterAttributes.Header.Type = NDIS_OBJECT_TYPE_FILTER_ATTRIBUTES;
    filterAttributes.Header.Size = sizeof(NDIS_FILTER_ATTRIBUTES);
    filterAttributes.Header.Revision = NDIS_FILTER_ATTRIBUTES_REVISION_1;
    filterAttributes.Flags = 0;

    status = NdisFSetAttributes(NdisFilterHandle, adapterContext, &filterAttributes);
    if (status != NDIS_STATUS_SUCCESS) {
        NdisFreeSpinLock(&adapterContext->Lock);
        ExFreePoolWithTag(adapterContext, NETOBS_POOL_TAG);
        return status;
    }

    NdisAcquireSpinLock(&g_NetObsContext.AdapterListLock);
    InsertTailList(&g_NetObsContext.AdapterList, &adapterContext->Link);
    NdisReleaseSpinLock(&g_NetObsContext.AdapterListLock);

    return NDIS_STATUS_SUCCESS;
}

VOID
FilterDetach(
    _In_ NDIS_HANDLE FilterModuleContext
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext = (PNETOBS_ADAPTER_CONTEXT)FilterModuleContext;

    NdisAcquireSpinLock(&g_NetObsContext.AdapterListLock);
    RemoveEntryList(&adapterContext->Link);
    NdisReleaseSpinLock(&g_NetObsContext.AdapterListLock);

    NdisFreeSpinLock(&adapterContext->Lock);
    ExFreePoolWithTag(adapterContext, NETOBS_POOL_TAG);
}

NDIS_STATUS
FilterRestart(
    _In_ NDIS_HANDLE FilterModuleContext,
    _In_ PNDIS_FILTER_RESTART_PARAMETERS RestartParameters
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext = (PNETOBS_ADAPTER_CONTEXT)FilterModuleContext;
    UNREFERENCED_PARAMETER(RestartParameters);

    // Best-effort initial stats read; the control device also queries on
    // demand, but priming here means /api/v1/interfaces has real numbers
    // immediately after the filter comes up, not just after the first poll.
    NetObsQueryAdapterStats(adapterContext);

    return NDIS_STATUS_SUCCESS;
}

NDIS_STATUS
FilterPause(
    _In_ NDIS_HANDLE FilterModuleContext,
    _In_ PNDIS_FILTER_PAUSE_PARAMETERS PauseParameters
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext = (PNETOBS_ADAPTER_CONTEXT)FilterModuleContext;
    UNREFERENCED_PARAMETER(PauseParameters);

    InterlockedExchange(&adapterContext->PauseRequested, 1);
    return NDIS_STATUS_SUCCESS;
}

// --- Pure passthrough data path -------------------------------------------
// This filter never inspects or modifies traffic (see filter.h header
// comment / docs/roadmap.md "fora de escopo permanente"). Every data,
// status, and OID path below simply forwards to the next layer.

VOID
FilterSendNetBufferLists(
    _In_ NDIS_HANDLE FilterModuleContext,
    _In_ PNET_BUFFER_LIST NetBufferLists,
    _In_ NDIS_PORT_NUMBER PortNumber,
    _In_ ULONG SendFlags
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext = (PNETOBS_ADAPTER_CONTEXT)FilterModuleContext;
    NdisFSendNetBufferLists(adapterContext->FilterHandle, NetBufferLists, PortNumber, SendFlags);
}

VOID
FilterSendNetBufferListsComplete(
    _In_ NDIS_HANDLE FilterModuleContext,
    _In_ PNET_BUFFER_LIST NetBufferLists,
    _In_ ULONG SendCompleteFlags
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext = (PNETOBS_ADAPTER_CONTEXT)FilterModuleContext;
    NdisFSendNetBufferListsComplete(adapterContext->FilterHandle, NetBufferLists, SendCompleteFlags);
}

VOID
FilterReceiveNetBufferLists(
    _In_ NDIS_HANDLE FilterModuleContext,
    _In_ PNET_BUFFER_LIST NetBufferLists,
    _In_ NDIS_PORT_NUMBER PortNumber,
    _In_ ULONG NumberOfNetBufferLists,
    _In_ ULONG ReceiveFlags
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext = (PNETOBS_ADAPTER_CONTEXT)FilterModuleContext;
    ULONG returnFlags = 0;

    if (NDIS_TEST_RECEIVE_CAN_PEND(ReceiveFlags)) {
        returnFlags |= NDIS_RECEIVE_FLAGS_DISPATCH_LEVEL & ReceiveFlags;
    }

    NdisFIndicateReceiveNetBufferLists(
        adapterContext->FilterHandle,
        NetBufferLists,
        PortNumber,
        NumberOfNetBufferLists,
        ReceiveFlags);
}

VOID
FilterReturnNetBufferLists(
    _In_ NDIS_HANDLE FilterModuleContext,
    _In_ PNET_BUFFER_LIST NetBufferLists,
    _In_ ULONG ReturnFlags
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext = (PNETOBS_ADAPTER_CONTEXT)FilterModuleContext;
    NdisFReturnNetBufferLists(adapterContext->FilterHandle, NetBufferLists, ReturnFlags);
}

NDIS_STATUS
FilterOidRequest(
    _In_ NDIS_HANDLE FilterModuleContext,
    _In_ PNDIS_OID_REQUEST Request
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext = (PNETOBS_ADAPTER_CONTEXT)FilterModuleContext;

    // Passive observation: peek at generic-statistics/link-state OIDs on
    // their way past, but never alter them or the completion status.
    if (Request->RequestType == NdisRequestQueryInformation) {
        switch (Request->DATA.QUERY_INFORMATION.Oid) {
        case OID_GEN_LINK_SPEED:
        case OID_GEN_MEDIA_CONNECT_STATUS:
        case OID_GEN_STATISTICS:
            NetObsQueryAdapterStats(adapterContext);
            break;
        default:
            break;
        }
    }

    return NdisFOidRequest(adapterContext->FilterHandle, Request);
}

VOID
FilterOidRequestComplete(
    _In_ NDIS_HANDLE FilterModuleContext,
    _In_ PNDIS_OID_REQUEST Request,
    _In_ NDIS_STATUS Status
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext = (PNETOBS_ADAPTER_CONTEXT)FilterModuleContext;

    // Self-originated queries (NetObsQueryOid, telemetry.c) complete here
    // too — NDIS routes every OID completion for this filter through this
    // handler regardless of who issued the request. Those must NOT be
    // forwarded upstream: there is no corresponding upper-layer request.
    if (NetObsIsInternalOid(Request)) {
        NetObsCompleteInternalOid(Request, Status);
        return;
    }

    NdisFOidRequestComplete(adapterContext->FilterHandle, Request, Status);
}

VOID
FilterCancelOidRequest(
    _In_ NDIS_HANDLE FilterModuleContext,
    _In_ PVOID RequestId
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext = (PNETOBS_ADAPTER_CONTEXT)FilterModuleContext;
    NdisFCancelOidRequest(adapterContext->FilterHandle, RequestId);
}

VOID
FilterStatus(
    _In_ NDIS_HANDLE FilterModuleContext,
    _In_ PNDIS_STATUS_INDICATION StatusIndication
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext = (PNETOBS_ADAPTER_CONTEXT)FilterModuleContext;

    if (StatusIndication->StatusCode == NDIS_STATUS_LINK_STATE) {
        PNDIS_LINK_STATE linkState = (PNDIS_LINK_STATE)StatusIndication->StatusBuffer;
        ULONG operStatus;

        MapOperStatus(linkState->MediaConnectState, &operStatus);

        NdisAcquireSpinLock(&adapterContext->Lock);
        adapterContext->LastStats.OperStatus = operStatus;
        adapterContext->LastStats.LinkSpeedBps = linkState->RcvLinkSpeed;
        NdisReleaseSpinLock(&adapterContext->Lock);
    }

    NdisFIndicateStatus(adapterContext->FilterHandle, StatusIndication);
}

NDIS_STATUS
FilterNetPnPEvent(
    _In_ NDIS_HANDLE FilterModuleContext,
    _In_ PNET_PNP_EVENT_NOTIFICATION NetPnPEventNotification
    )
{
    PNETOBS_ADAPTER_CONTEXT adapterContext = (PNETOBS_ADAPTER_CONTEXT)FilterModuleContext;
    return NdisFNetPnPEvent(adapterContext->FilterHandle, NetPnPEventNotification);
}

static VOID
MapOperStatus(
    _In_ NDIS_MEDIA_CONNECT_STATE ConnectState,
    _Out_ PULONG OperStatus
    )
{
    // Values match collector_core::OperStatus's discriminant order
    // (crates/collector-core/src/model.rs) so the user-mode backend can
    // cast directly: 0=Up 1=Down 2=Testing 3=Unknown 4=Dormant 5=NotPresent 6=LowerLayerDown.
    switch (ConnectState) {
    case MediaConnectStateConnected:
        *OperStatus = 0;
        break;
    case MediaConnectStateDisconnected:
        *OperStatus = 1;
        break;
    default:
        *OperStatus = 3;
        break;
    }
}
