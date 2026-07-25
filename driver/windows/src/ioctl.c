#include "..\inc\filter.h"

static DRIVER_DISPATCH NetObsDispatch;
static DRIVER_DISPATCH NetObsDeviceIoControl;

VOID
NetObsBuildSnapshot(
    _Out_ PNETOBS_SNAPSHOT Snapshot
    )
{
    PLIST_ENTRY entry;

    RtlZeroMemory(Snapshot, sizeof(*Snapshot));

    NdisAcquireSpinLock(&g_NetObsContext.AdapterListLock);

    for (entry = g_NetObsContext.AdapterList.Flink;
         entry != &g_NetObsContext.AdapterList && Snapshot->AdapterCount < NETOBS_MAX_ADAPTERS;
         entry = entry->Flink)
    {
        PNETOBS_ADAPTER_CONTEXT adapterContext = CONTAINING_RECORD(entry, NETOBS_ADAPTER_CONTEXT, Link);

        NdisAcquireSpinLock(&adapterContext->Lock);
        Snapshot->Adapters[Snapshot->AdapterCount] = adapterContext->LastStats;
        NdisReleaseSpinLock(&adapterContext->Lock);

        Snapshot->AdapterCount++;
    }

    NdisReleaseSpinLock(&g_NetObsContext.AdapterListLock);
}

static NTSTATUS
NetObsDeviceIoControl(
    _In_ PDEVICE_OBJECT DeviceObject,
    _In_ PIRP Irp
    )
{
    PIO_STACK_LOCATION irpStack = IoGetCurrentIrpStackLocation(Irp);
    NTSTATUS status = STATUS_SUCCESS;
    ULONG_PTR bytesReturned = 0;

    UNREFERENCED_PARAMETER(DeviceObject);

    switch (irpStack->Parameters.DeviceIoControl.IoControlCode) {
    case IOCTL_NETOBS_GET_SNAPSHOT: {
        ULONG outLength = irpStack->Parameters.DeviceIoControl.OutputBufferLength;
        if (outLength < sizeof(NETOBS_SNAPSHOT)) {
            status = STATUS_BUFFER_TOO_SMALL;
            break;
        }
        // METHOD_BUFFERED: Irp->AssociatedIrp.SystemBuffer is a kernel-owned
        // bounce buffer already sized/validated by the I/O manager — never a
        // raw user-mode pointer, matching the validation rule in
        // docs/phase2-kernel-driver-design.md.
        NetObsBuildSnapshot((PNETOBS_SNAPSHOT)Irp->AssociatedIrp.SystemBuffer);
        bytesReturned = sizeof(NETOBS_SNAPSHOT);
        break;
    }

    case IOCTL_NETOBS_GET_ADAPTER_COUNT: {
        NETOBS_SNAPSHOT snapshot;
        ULONG outLength = irpStack->Parameters.DeviceIoControl.OutputBufferLength;
        if (outLength < sizeof(ULONG)) {
            status = STATUS_BUFFER_TOO_SMALL;
            break;
        }
        NetObsBuildSnapshot(&snapshot);
        *(PULONG)Irp->AssociatedIrp.SystemBuffer = snapshot.AdapterCount;
        bytesReturned = sizeof(ULONG);
        break;
    }

    default:
        status = STATUS_INVALID_DEVICE_REQUEST;
        break;
    }

    Irp->IoStatus.Status = status;
    Irp->IoStatus.Information = bytesReturned;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return status;
}

static NTSTATUS
NetObsDispatch(
    _In_ PDEVICE_OBJECT DeviceObject,
    _In_ PIRP Irp
    )
{
    UNREFERENCED_PARAMETER(DeviceObject);

    Irp->IoStatus.Status = STATUS_SUCCESS;
    Irp->IoStatus.Information = 0;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return STATUS_SUCCESS;
}

NDIS_STATUS
NetObsCreateControlDevice(VOID)
{
    NDIS_STRING deviceName = RTL_CONSTANT_STRING(NETOBS_DEVICE_NAME);
    NDIS_STRING symlinkName = RTL_CONSTANT_STRING(NETOBS_SYMLINK_NAME);
    NDIS_DEVICE_OBJECT_ATTRIBUTES deviceAttrs;
    PDRIVER_DISPATCH dispatchTable[IRP_MJ_MAXIMUM_FUNCTION + 1];

    RtlZeroMemory(dispatchTable, sizeof(dispatchTable));
    dispatchTable[IRP_MJ_CREATE] = NetObsDispatch;
    dispatchTable[IRP_MJ_CLOSE] = NetObsDispatch;
    dispatchTable[IRP_MJ_DEVICE_CONTROL] = NetObsDeviceIoControl;

    RtlZeroMemory(&deviceAttrs, sizeof(deviceAttrs));
    deviceAttrs.Header.Type = NDIS_OBJECT_TYPE_DEVICE_OBJECT_ATTRIBUTES;
    deviceAttrs.Header.Revision = NDIS_DEVICE_OBJECT_ATTRIBUTES_REVISION_1;
    deviceAttrs.Header.Size = sizeof(deviceAttrs);
    deviceAttrs.DeviceName = &deviceName;
    deviceAttrs.SymbolicName = &symlinkName;
    deviceAttrs.MajorFunctions = dispatchTable;
    deviceAttrs.ExtensionSize = 0;

    return NdisRegisterDeviceEx(
        g_NetObsContext.DriverHandle,
        &deviceAttrs,
        &g_NetObsContext.ControlDeviceObject,
        &g_NetObsContext.NdisDeviceHandle);
}

VOID
NetObsDeleteControlDevice(VOID)
{
    if (g_NetObsContext.NdisDeviceHandle != NULL) {
        NdisDeregisterDeviceEx(g_NetObsContext.NdisDeviceHandle);
        g_NetObsContext.NdisDeviceHandle = NULL;
    }
}
