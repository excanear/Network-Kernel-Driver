/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _NETOBS_H
#define _NETOBS_H

#include <linux/netdevice.h>
#include <net/genetlink.h>
#include "netobs_uapi.h"

extern struct genl_family netobs_family;

int netobs_netlink_init(void);
void netobs_netlink_exit(void);

/* Fills one NETOBS_CMD_GET_SNAPSHOT reply message for a single net_device.
 * Returns 0 on success, or a negative errno (e.g. -EMSGSIZE if skb is full,
 * in which case the caller should send what's built so far and retry this
 * device in a fresh skb — same pattern as rtnetlink dumps).
 */
int netobs_fill_device(struct sk_buff *skb, struct net_device *dev,
		       u32 portid, u32 seq, int flags);

/* register_netdevice_notifier callback wired up in netobs_main.c, forwards
 * NETDEV_UP/NETDEV_DOWN/NETDEV_CHANGE as NETOBS_CMD_LINK_EVENT multicasts.
 */
int netobs_notifier_init(void);
void netobs_notifier_exit(void);

#endif /* _NETOBS_H */
