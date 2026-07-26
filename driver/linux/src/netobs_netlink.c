// SPDX-License-Identifier: GPL-2.0
/*
 * Generic Netlink family registration and the NETOBS_CMD_GET_SNAPSHOT dump
 * handler. See docs/phase2-kernel-driver-design.md for the wire contract.
 */
#include <linux/module.h>
#include <linux/netdevice.h>
#include <net/genetlink.h>
#include <net/net_namespace.h>

#include "../include/netobs.h"

static int netobs_dump_snapshot(struct sk_buff *skb, struct netlink_callback *cb)
{
	struct net_device *dev;
	int idx = 0;
	int skip = cb->args[0];

	rcu_read_lock();
	for_each_netdev_rcu(&init_net, dev) {
		if (idx++ < skip)
			continue;

		if (netobs_fill_device(skb, dev,
					NETLINK_CB(cb->skb).portid,
					cb->nlh->nlmsg_seq,
					NLM_F_MULTI) < 0)
			break; /* skb full — resume from this device next call */
	}
	rcu_read_unlock();

	cb->args[0] = idx;
	return skb->len;
}

static const struct genl_ops netobs_ops[] = {
	{
		.cmd = NETOBS_CMD_GET_SNAPSHOT,
		.dumpit = netobs_dump_snapshot,
		.flags = 0, /* readable by any user — matches least-privilege design */
	},
};

static const struct genl_multicast_group netobs_mcgrps[] = {
	{ .name = NETOBS_MCGRP_LINK_EVENTS, },
};

struct genl_family netobs_family = {
	.name = NETOBS_GENL_FAMILY_NAME,
	.version = NETOBS_GENL_VERSION,
	.maxattr = NETOBS_A_MAX,
	.module = THIS_MODULE,
	.ops = netobs_ops,
	.n_ops = ARRAY_SIZE(netobs_ops),
	.mcgrps = netobs_mcgrps,
	.n_mcgrps = ARRAY_SIZE(netobs_mcgrps),
};

int netobs_netlink_init(void)
{
	return genl_register_family(&netobs_family);
}

void netobs_netlink_exit(void)
{
	genl_unregister_family(&netobs_family);
}
