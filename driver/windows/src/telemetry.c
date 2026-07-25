#include "..\inc\filter.h"

// Sentinel written into an internally-originated NDIS_OID_REQUEST's
// SourceReserved[0] so FilterOidRequestComplete (filter.c) can tell "this
// is my own query completing" apart from "an upper layer's passthrough
// request is completing" — the two must be handled completely differently
// (signal-and-free vs NdisFOidRequestComplete). See NetObsIsInternalOid.
#define NETOBS_INTERNAL_OID_MARKER ((PVOID)(ULONG_PTR)0x4E4F4253) // "NOBS"

typedef struct _NETOBS_OID_REQUEST_CONTEXT {
    NDIS_EVENT   Event;
    NDIS_STATUS  Status;
} NETOBS_OID_REQUEST_CONTEXT, *PNETOBS_OID_REQUEST_CONTEXT;

BOOLEAN
NetObsIsInternalOid(
    _In_ PNDIS_OID_REQUEST Request
    )
{
    PVOID *reserved = (PVOID *)Request->SourceReserved;
    return reserved[0] == NETOBS_INTERNAL_OID_MARKER;
}

VOID
NetObsCompleteInternalOid(
    _In_ PNDIS_OID_REQUEST Request,
    _In_ NDIS_STATUS Status
    )
{
    PVOID *reserved = (PVOID *)Request->SourceReserved;
    PNETOBS_OID_REQUEST_CONTEXT ctx = (PNETOBS_OID_REQUEST_CONTEXT)reserved[1];

    ctx->Status = Status;
    NdisSetEvent(&ctx->Event);
}

static NDIS_STATUS
NetObsQueryOid(
    _In_ PNETOBS_ADAPTER_CONTEXT AdapterContext,
    _In_ NDIS_OID Oid,
    _Out_writes_bytes_(BufferLength) PVOID Buffer,
    _In_ ULONG BufferLength,
    _Out_ PULONG BytesWritten
    )
{
    NDIS_OID_REQUEST request;
    NETOBS_OID_REQUEST_CONTEXT ctx;
    PVOID *reserved;
    NDIS_STATUS status;

    RtlZeroMemory(&request, sizeof(request));
    request.Header.Type = NDIS_OBJECT_TYPE_OID_REQUEST;
    request.Header.Size = NDIS_SIZEOF_OID_REQUEST_REVISION_1;
    request.Header.Revision = NDIS_OID_REQUEST_REVISION_1;
    request.RequestType = NdisRequestQueryInformation;
    request.DATA.QUERY_INFORMATION.Oid = Oid;
    request.DATA.QUERY_INFORMATION.InformationBuffer = Buffer;
    request.DATA.QUERY_INFORMATION.InformationBufferLength = BufferLength;

    reserved = (PVOID *)request.SourceReserved;
    reserved[0] = NETOBS_INTERNAL_OID_MARKER;
    reserved[1] = &ctx;

    NdisInitializeEvent(&ctx.Event);
    ctx.Status = NDIS_STATUS_PENDING;

    status = NdisFOidRequest(AdapterContext->FilterHandle, &request);
    if (status == NDIS_STATUS_PENDING) {
        // FilterOidRequestComplete (filter.c) recognizes the marker above,
        // signals this event, and does NOT forward to NdisFOidRequestComplete
        // — there is no upstream request to complete for a self-originated
        // query. 5s is generous; a well-behaved miniport completes in µs-ms.
        NdisWaitEvent(&ctx.Event, 5000);
        status = ctx.Status;
    }

    *BytesWritten = request.DATA.QUERY_INFORMATION.BytesWritten;
    return status;
}

NDIS_STATUS
NetObsQueryAdapterStats(
    _In_ PNETOBS_ADAPTER_CONTEXT AdapterContext
    )
{
    NDIS_STATISTICS_INFO statsInfo;
    ULONG bytesWritten = 0;
    NDIS_STATUS status;

    RtlZeroMemory(&statsInfo, sizeof(statsInfo));
    statsInfo.Header.Type = NDIS_OBJECT_TYPE_DEFAULT;
    statsInfo.Header.Revision = NDIS_STATISTICS_INFO_REVISION_1;
    statsInfo.Header.Size = NDIS_SIZEOF_STATISTICS_INFO_REVISION_1;

    status = NetObsQueryOid(
        AdapterContext, OID_GEN_STATISTICS, &statsInfo, sizeof(statsInfo), &bytesWritten);

    NdisAcquireSpinLock(&AdapterContext->Lock);
    AdapterContext->LastStats.IfIndex = AdapterContext->IfIndex;

    if (status == NDIS_STATUS_SUCCESS) {
        if (statsInfo.SupportedStatistics & NDIS_STATISTICS_FLAGS_VALID_DIRECTED_FRAMES_XMIT) {
            AdapterContext->LastStats.TxPackets = statsInfo.ifHCOutUcastPkts +
                statsInfo.ifHCOutMulticastPkts + statsInfo.ifHCOutBroadcastPkts;
        }
        if (statsInfo.SupportedStatistics & NDIS_STATISTICS_FLAGS_VALID_DIRECTED_FRAMES_RCV) {
            AdapterContext->LastStats.RxPackets = statsInfo.ifHCInUcastPkts +
                statsInfo.ifHCInMulticastPkts + statsInfo.ifHCInBroadcastPkts;
        }
        if (statsInfo.SupportedStatistics & NDIS_STATISTICS_FLAGS_VALID_BYTES_RCV) {
            AdapterContext->LastStats.RxBytes = statsInfo.ifHCInOctets;
        }
        if (statsInfo.SupportedStatistics & NDIS_STATISTICS_FLAGS_VALID_BYTES_XMIT) {
            AdapterContext->LastStats.TxBytes = statsInfo.ifHCOutOctets;
        }
        if (statsInfo.SupportedStatistics & NDIS_STATISTICS_FLAGS_VALID_RCV_ERROR) {
            AdapterContext->LastStats.RxErrors = statsInfo.ifInErrors;
        }
        if (statsInfo.SupportedStatistics & NDIS_STATISTICS_FLAGS_VALID_XMIT_ERROR) {
            AdapterContext->LastStats.TxErrors = statsInfo.ifOutErrors;
        }
        if (statsInfo.SupportedStatistics & NDIS_STATISTICS_FLAGS_VALID_RCV_DISCARDS) {
            AdapterContext->LastStats.RxDiscards = statsInfo.ifInDiscards;
        }
        if (statsInfo.SupportedStatistics & NDIS_STATISTICS_FLAGS_VALID_XMIT_DISCARDS) {
            AdapterContext->LastStats.TxDiscards = statsInfo.ifOutDiscards;
        }
    }
    NdisReleaseSpinLock(&AdapterContext->Lock);

    return status;
}
