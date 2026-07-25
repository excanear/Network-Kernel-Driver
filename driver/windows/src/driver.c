#include "..\inc\filter.h"

NETOBS_GLOBAL_CONTEXT g_NetObsContext = { 0 };

NDIS_STATUS
DriverEntry(
    _In_ PDRIVER_OBJECT DriverObject,
    _In_ PUNICODE_STRING RegistryPath
    )
{
    NDIS_STATUS status;
    NDIS_FILTER_DRIVER_CHARACTERISTICS filterChars;

    RtlZeroMemory(&g_NetObsContext, sizeof(g_NetObsContext));
    NdisAllocateSpinLock(&g_NetObsContext.AdapterListLock);
    InitializeListHead(&g_NetObsContext.AdapterList);

    RtlZeroMemory(&filterChars, sizeof(filterChars));
    filterChars.Header.Type = NDIS_OBJECT_TYPE_FILTER_DRIVER_CHARACTERISTICS;
    filterChars.Header.Size = sizeof(NDIS_FILTER_DRIVER_CHARACTERISTICS);
    filterChars.Header.Revision = NDIS_FILTER_CHARACTERISTICS_REVISION_2;

    filterChars.MajorNdisVersion = 6;
    filterChars.MinorNdisVersion = 30;
    filterChars.MajorDriverVersion = 1;
    filterChars.MinorDriverVersion = 0;

    filterChars.FriendlyName.Buffer = NETOBS_FILTER_FRIENDLY_NAME;
    filterChars.FriendlyName.Length = (USHORT)wcslen(NETOBS_FILTER_FRIENDLY_NAME) * sizeof(WCHAR);
    filterChars.FriendlyName.MaximumLength = filterChars.FriendlyName.Length;

    filterChars.UniqueName.Buffer = NETOBS_FILTER_UNIQUE_NAME;
    filterChars.UniqueName.Length = (USHORT)wcslen(NETOBS_FILTER_UNIQUE_NAME) * sizeof(WCHAR);
    filterChars.UniqueName.MaximumLength = filterChars.UniqueName.Length;

    filterChars.ServiceName.Buffer = NETOBS_FILTER_SERVICE_NAME;
    filterChars.ServiceName.Length = (USHORT)wcslen(NETOBS_FILTER_SERVICE_NAME) * sizeof(WCHAR);
    filterChars.ServiceName.MaximumLength = filterChars.ServiceName.Length;

    filterChars.SetOptionsHandler = NULL;
    filterChars.AttachHandler = FilterAttach;
    filterChars.DetachHandler = FilterDetach;
    filterChars.RestartHandler = FilterRestart;
    filterChars.PauseHandler = FilterPause;
    filterChars.OidRequestHandler = FilterOidRequest;
    filterChars.OidRequestCompleteHandler = FilterOidRequestComplete;
    filterChars.CancelOidRequestHandler = FilterCancelOidRequest;
    filterChars.SendNetBufferListsHandler = FilterSendNetBufferLists;
    filterChars.SendNetBufferListsCompleteHandler = FilterSendNetBufferListsComplete;
    filterChars.ReceiveNetBufferListsHandler = FilterReceiveNetBufferLists;
    filterChars.ReturnNetBufferListsHandler = FilterReturnNetBufferLists;
    filterChars.StatusHandler = FilterStatus;
    filterChars.NetPnPEventHandler = FilterNetPnPEvent;

    DriverObject->DriverUnload = FilterDriverUnload;

    status = NdisFRegisterFilterDriver(
        DriverObject,
        (NDIS_HANDLE)&g_NetObsContext,
        &filterChars,
        &g_NetObsContext.DriverHandle);

    if (status != NDIS_STATUS_SUCCESS) {
        return status;
    }

    status = NetObsCreateControlDevice();
    if (status != NDIS_STATUS_SUCCESS) {
        NdisFDeregisterFilterDriver(g_NetObsContext.DriverHandle);
        return status;
    }

    return NDIS_STATUS_SUCCESS;
}

VOID
FilterDriverUnload(
    _In_ PDRIVER_OBJECT DriverObject
    )
{
    UNREFERENCED_PARAMETER(DriverObject);

    NetObsDeleteControlDevice();

    if (g_NetObsContext.DriverHandle != NULL) {
        NdisFDeregisterFilterDriver(g_NetObsContext.DriverHandle);
    }
}
