// SPDX-License-Identifier: GPL-2.0
/*
 * Per-device stat collection for NETOBS_CMD_GET_SNAPSHOT. Pure read-only
 * observation via the standard netdev stats API (dev_get_stats) and carrier
 * state (netif_carrier_ok) — no packet inspection, no filtering hooks.
 */
#include <linux/module.h>
#include <linux/netdevice.h>
#include <linux/ethtool.h>
#include <net/genetlink.h>

#include "../include/netobs.h"

/* Maps Linux carrier/operstate into the same discriminant order as
 * collector_core::OperStatus (crates/collector-core/src/model.rs) so the
 * user-mode backend can use the value directly without a Linux-specific
 * translation table — the same convention the Windows filter driver's
 * MapOperStatus (driver/windows/src/filter.c) follows.
 */
static u32 netobs_oper_status(const struct net_device *dev)
{
	if (!(dev->flags & IFF_UP))
		return 1; /* Down */

	switch (dev->operstate) {
	case IF_OPER_UP:
		return 0; /* Up */
	case IF_OPER_DOWN:
		return 1; /* Down */
	case IF_OPER_TESTING:
		return 2; /* Testing */
	case IF_OPER_DORMANT:
		return 4; /* Dormant */
	case IF_OPER_NOTPRESENT:
		return 5; /* NotPresent */
	case IF_OPER_LOWERLAYERDOWN:
		return 6; /* LowerLayerDown */
	default:
		return 3; /* Unknown */
	}
}

static u32 netobs_link_speed_mbps(struct net_device *dev)
{
	struct ethtool_link_ksettings cmd;

	if (!dev->ethtool_ops || !dev->ethtool_ops->get_link_ksettings)
		return 0;
	if (!netif_running(dev) || !netif_carrier_ok(dev))
		return 0;
	if (dev->ethtool_ops->get_link_ksettings(dev, &cmd) != 0)
		return 0;
	if (cmd.base.speed == 0 || cmd.base.speed == (u32)SPEED_UNKNOWN)
		return 0;
	return cmd.base.speed;
}

int netobs_fill_device(struct sk_buff *skb, struct net_device *dev,
		       u32 portid, u32 seq, int flags)
{
	struct rtnl_link_stats64 storage;
	const struct rtnl_link_stats64 *stats;
	void *hdr;

	hdr = genlmsg_put(skb, portid, seq, &netobs_family, flags,
			  NETOBS_CMD_GET_SNAPSHOT);
	if (!hdr)
		return -EMSGSIZE;

	stats = dev_get_stats(dev, &storage);

	if (nla_put_u32(skb, NETOBS_A_IF_INDEX, (u32)dev->ifindex) ||
	    nla_put_string(skb, NETOBS_A_IF_NAME, dev->name) ||
	    nla_put_u32(skb, NETOBS_A_OPER_STATUS, netobs_oper_status(dev)) ||
	    nla_put_u32(skb, NETOBS_A_LINK_SPEED_MBPS, netobs_link_speed_mbps(dev)) ||
	    nla_put_u64_64bit(skb, NETOBS_A_RX_BYTES, stats->rx_bytes, NETOBS_A_UNSPEC) ||
	    nla_put_u64_64bit(skb, NETOBS_A_TX_BYTES, stats->tx_bytes, NETOBS_A_UNSPEC) ||
	    nla_put_u64_64bit(skb, NETOBS_A_RX_PACKETS, stats->rx_packets, NETOBS_A_UNSPEC) ||
	    nla_put_u64_64bit(skb, NETOBS_A_TX_PACKETS, stats->tx_packets, NETOBS_A_UNSPEC) ||
	    nla_put_u64_64bit(skb, NETOBS_A_RX_ERRORS, stats->rx_errors, NETOBS_A_UNSPEC) ||
	    nla_put_u64_64bit(skb, NETOBS_A_TX_ERRORS, stats->tx_errors, NETOBS_A_UNSPEC) ||
	    nla_put_u64_64bit(skb, NETOBS_A_RX_DROPS, stats->rx_dropped, NETOBS_A_UNSPEC) ||
	    nla_put_u64_64bit(skb, NETOBS_A_TX_DROPS, stats->tx_dropped, NETOBS_A_UNSPEC)) {
		genlmsg_cancel(skb, hdr);
		return -EMSGSIZE;
	}

	genlmsg_end(skb, hdr);
	return 0;
}
