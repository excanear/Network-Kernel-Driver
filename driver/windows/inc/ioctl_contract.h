/*
 * Shared IOCTL contract between the NetworkObservatory NDIS filter driver
 * and the user-mode `collector-kernel-windows` backend (Phase 2/L).
 *
 * Kept deliberately small and POD-only (no pointers, no variable-length
 * fields) so validation on the kernel side is trivial: every IOCTL buffer
 * is checked for exact expected size before use, and no user-mode pointer
 * is ever dereferenced directly in kernel context (METHOD_BUFFERED only).
 */
#pragma once

#include <initguid.h>

// {2C1B9F3E-7F2A-4C1D-9B0E-6B7B6E9E9C10}
DEFINE_GUID(GUID_DEVCLASS_NETOBS_CONTROL,
    0x2c1b9f3e, 0x7f2a, 0x4c1d, 0x9b, 0x0e, 0x6b, 0x7b, 0x6e, 0x9e, 0x9c, 0x10);

#define NETOBS_DEVICE_NAME      L"\\Device\\NetObsFilter"
#define NETOBS_SYMLINK_NAME     L"\\DosDevices\\NetObsFilter"
#define NETOBS_WIN32_DEVICE     L"\\\\.\\NetObsFilter"

#define FILE_DEVICE_NETOBS 0x8010

#define IOCTL_NETOBS_GET_SNAPSHOT \
    CTL_CODE(FILE_DEVICE_NETOBS, 0x800, METHOD_BUFFERED, FILE_ANY_ACCESS)

#define IOCTL_NETOBS_GET_ADAPTER_COUNT \
    CTL_CODE(FILE_DEVICE_NETOBS, 0x801, METHOD_BUFFERED, FILE_ANY_ACCESS)

#pragma pack(push, 1)

// Mirrors the fields of InterfaceStats (crates/collector-core/src/model.rs)
// that are actually obtainable from NDIS OID queries at the filter's
// attach point — a strict subset of the full model. The user-mode backend
// fills in the rest (name/description/IPs) via IP Helper API and merges by
// if_index, matching the fallback story in docs/phase2-kernel-driver-design.md.
typedef struct _NETOBS_ADAPTER_STATS {
    ULONG   IfIndex;
    ULONG   OperStatus;          // NDIS_MEDIA_CONNECT_STATE mapped to Phase 1 OperStatus values
    ULONG64 LinkSpeedBps;
    ULONG64 RxBytes;
    ULONG64 TxBytes;
    ULONG64 RxPackets;
    ULONG64 TxPackets;
    ULONG64 RxErrors;
    ULONG64 TxErrors;
    ULONG64 RxDiscards;
    ULONG64 TxDiscards;
} NETOBS_ADAPTER_STATS, *PNETOBS_ADAPTER_STATS;

#define NETOBS_MAX_ADAPTERS 32

typedef struct _NETOBS_SNAPSHOT {
    ULONG                 AdapterCount;
    NETOBS_ADAPTER_STATS  Adapters[NETOBS_MAX_ADAPTERS];
} NETOBS_SNAPSHOT, *PNETOBS_SNAPSHOT;

#pragma pack(pop)
