// SPDX-License-Identifier: GPL-2.0
/*
 * NetworkObservatory Linux kernel module (Phase 2/M) — module init/exit,
 * genetlink family registration, and link-event notifications.
 *
 * Scope discipline, same as the Windows filter driver
 * (driver/windows/inc/filter.h): this module is a pure OBSERVER. It never
 * registers a netfilter hook, never touches an skb in the data path, and
 * never modifies device state. It only reads netdev stats/carrier state and
 * forwards NETDEV_* notifier events as read-only Netlink notifications. See
 * docs/roadmap.md's "fora de escopo permanente" section.
 */
#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/netdevice.h>
#include <linux/notifier.h>
#include <net/genetlink.h>

#include "../include/netobs.h"

MODULE_LICENSE("GPL");
MODULE_AUTHOR("Network Observatory Contributors");
MODULE_DESCRIPTION("Read-only network interface observability (stats + link state via Netlink)");
MODULE_VERSION("0.1.0");

static void netobs_notify_link_event(struct net_device *dev)
{
	struct sk_buff *skb;
	void *hdr;

	skb = genlmsg_new(NLMSG_GOODSIZE, GFP_KERNEL);
	if (!skb)
		return;

	hdr = genlmsg_put(skb, 0, 0, &netobs_family, 0, NETOBS_CMD_LINK_EVENT);
	if (!hdr)
		goto out_free;

	if (nla_put_u32(skb, NETOBS_A_IF_INDEX, (u32)dev->ifindex) ||
	    nla_put_string(skb, NETOBS_A_IF_NAME, dev->name)) {
		genlmsg_cancel(skb, hdr);
		goto out_free;
	}

	genlmsg_end(skb, hdr);

	/* No subscribers is the common case and not an error. */
	genlmsg_multicast(&netobs_family, skb, 0, 0, GFP_KERNEL);
	return;

out_free:
	nlmsg_free(skb);
}

static int netobs_netdev_event(struct notifier_block *nb, unsigned long event, void *ptr)
{
	struct net_device *dev = netdev_notifier_info_to_dev(ptr);

	switch (event) {
	case NETDEV_UP:
	case NETDEV_DOWN:
	case NETDEV_CHANGE:
		netobs_notify_link_event(dev);
		break;
	default:
		break;
	}
	return NOTIFY_DONE;
}

static struct notifier_block netobs_netdev_notifier = {
	.notifier_call = netobs_netdev_event,
};

int netobs_notifier_init(void)
{
	return register_netdevice_notifier(&netobs_netdev_notifier);
}

void netobs_notifier_exit(void)
{
	unregister_netdevice_notifier(&netobs_netdev_notifier);
}

static int __init netobs_init(void)
{
	int ret;

	ret = netobs_netlink_init();
	if (ret) {
		pr_err("netobs: failed to register genetlink family: %d\n", ret);
		return ret;
	}

	ret = netobs_notifier_init();
	if (ret) {
		pr_err("netobs: failed to register netdevice notifier: %d\n", ret);
		netobs_netlink_exit();
		return ret;
	}

	pr_info("netobs: loaded (genetlink family '%s')\n", NETOBS_GENL_FAMILY_NAME);
	return 0;
}

static void __exit netobs_exit(void)
{
	netobs_notifier_exit();
	netobs_netlink_exit();
	pr_info("netobs: unloaded\n");
}

module_init(netobs_init);
module_exit(netobs_exit);
