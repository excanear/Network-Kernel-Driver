/* SPDX-License-Identifier: GPL-2.0 */
/*
 * Shared Netlink (genetlink) contract between the NetworkObservatory Linux
 * kernel module and the user-mode `collector-kernel-linux` backend
 * (Phase 2/M). Mirrors the same InterfaceStats fields the Windows filter
 * driver's IOCTL contract exposes (driver/windows/inc/ioctl_contract.h) so
 * both platforms produce the same logical shape — see
 * docs/phase2-kernel-driver-design.md.
 *
 * Read-only by design: every command here queries state. There is no
 * command that modifies routing, filtering, or packet delivery — see the
 * "fora de escopo permanente" section of docs/roadmap.md.
 */
#ifndef _NETOBS_UAPI_H
#define _NETOBS_UAPI_H

#define NETOBS_GENL_FAMILY_NAME "netobs"
#define NETOBS_GENL_VERSION 1
#define NETOBS_MCGRP_LINK_EVENTS "netobs_link"

enum netobs_command {
	NETOBS_CMD_UNSPEC,
	NETOBS_CMD_GET_SNAPSHOT,	/* dump: one reply message per interface */
	NETOBS_CMD_LINK_EVENT,		/* multicast-only: link up/down notification */
	__NETOBS_CMD_MAX,
};
#define NETOBS_CMD_MAX (__NETOBS_CMD_MAX - 1)

enum netobs_attr {
	NETOBS_A_UNSPEC,
	NETOBS_A_IF_INDEX,		/* u32 */
	NETOBS_A_IF_NAME,		/* string, IFNAMSIZ */
	NETOBS_A_OPER_STATUS,		/* u32, matches collector_core::OperStatus discriminant */
	NETOBS_A_LINK_SPEED_MBPS,	/* u32, 0 if unknown */
	NETOBS_A_RX_BYTES,		/* u64 */
	NETOBS_A_TX_BYTES,		/* u64 */
	NETOBS_A_RX_PACKETS,		/* u64 */
	NETOBS_A_TX_PACKETS,		/* u64 */
	NETOBS_A_RX_ERRORS,		/* u64 */
	NETOBS_A_TX_ERRORS,		/* u64 */
	NETOBS_A_RX_DROPS,		/* u64 */
	NETOBS_A_TX_DROPS,		/* u64 */
	__NETOBS_A_MAX,
};
#define NETOBS_A_MAX (__NETOBS_A_MAX - 1)

#endif /* _NETOBS_UAPI_H */
