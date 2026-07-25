/*
 * Internal driver header for the NetworkObservatory NDIS 6.x Lightweight
 * Filter (LWF). See docs/phase2-kernel-driver-design.md for the overall
 * design and docs/architecture.md for how this fits the six-layer platform.
 *
 * Scope discipline: this filter is a pure OBSERVER. It attaches to the NDIS
 * stack to query adapter statistics/link state via OID requests and passes
 * every data/status/OID path through completely unmodified. It never
 * inspects, drops, delays, or rewrites a packet. That is a deliberate
 * architectural constraint, not just an implementation detail — see the
 * "fora de escopo permanente" section of docs/roadmap.md.
 */
#pragma once

#include <ndis.h>
#include <ntddk.h>
#include "..\inc\ioctl_contract.h"

#define NETOBS_FILTER_UNIQUE_NAME  L"{7E2E6F1B-9A3D-4B6E-8C1A-2F5D7E8B9A11}"
#define NETOBS_FILTER_FRIENDLY_NAME L"Network Observatory Filter"
#define NETOBS_FILTER_SERVICE_NAME L"NetObsFilter"

#define NETOBS_POOL_TAG 'sboN' // "Nobs" — used for all ExAllocatePool2 calls in this driver

typedef struct _NETOBS_ADAPTER_CONTEXT {
    LIST_ENTRY          Link;
    NDIS_HANDLE         FilterHandle;
    ULONG               IfIndex;
    NDIS_STRING         MiniportName;
    NDIS_SPIN_LOCK      Lock;
    NETOBS_ADAPTER_STATS LastStats;
    volatile LONG       PauseRequested; // NDIS_FILTER_PAUSE serialization guard
} NETOBS_ADAPTER_CONTEXT, *PNETOBS_ADAPTER_CONTEXT;

// Global, driver-wide state: the list of currently attached adapters and
// the control device used for user-mode IOCTL access (crates/collector-*).
typedef struct _NETOBS_GLOBAL_CONTEXT {
    NDIS_HANDLE       DriverHandle;
    NDIS_HANDLE       NdisDeviceHandle;
    PDEVICE_OBJECT    ControlDeviceObject;
    NDIS_SPIN_LOCK    AdapterListLock;
    LIST_ENTRY        AdapterList;
} NETOBS_GLOBAL_CONTEXT, *PNETOBS_GLOBAL_CONTEXT;

extern NETOBS_GLOBAL_CONTEXT g_NetObsContext;

// driver.c
DRIVER_INITIALIZE DriverEntry;
DRIVER_UNLOAD FilterDriverUnload;

// filter.c
FILTER_ATTACH FilterAttach;
FILTER_DETACH FilterDetach;
FILTER_RESTART FilterRestart;
FILTER_PAUSE FilterPause;
FILTER_SEND_NET_BUFFER_LISTS FilterSendNetBufferLists;
FILTER_SEND_NET_BUFFER_LISTS_COMPLETE FilterSendNetBufferListsComplete;
FILTER_RECEIVE_NET_BUFFER_LISTS FilterReceiveNetBufferLists;
FILTER_RETURN_NET_BUFFER_LISTS FilterReturnNetBufferLists;
FILTER_OID_REQUEST FilterOidRequest;
FILTER_OID_REQUEST_COMPLETE FilterOidRequestComplete;
FILTER_CANCEL_OID_REQUEST FilterCancelOidRequest;
FILTER_STATUS FilterStatus;
FILTER_NET_PNP_EVENT FilterNetPnPEvent;

// telemetry.c
NDIS_STATUS
NetObsQueryAdapterStats(
    _In_ PNETOBS_ADAPTER_CONTEXT AdapterContext
    );

BOOLEAN
NetObsIsInternalOid(
    _In_ PNDIS_OID_REQUEST Request
    );

VOID
NetObsCompleteInternalOid(
    _In_ PNDIS_OID_REQUEST Request,
    _In_ NDIS_STATUS Status
    );

// ioctl.c
NDIS_STATUS NetObsCreateControlDevice(VOID);
VOID NetObsDeleteControlDevice(VOID);
VOID NetObsBuildSnapshot(_Out_ PNETOBS_SNAPSHOT Snapshot);
