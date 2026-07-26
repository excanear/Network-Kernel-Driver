# SPDX-License-Identifier: GPL-2.0
# Out-of-tree kbuild Makefile for the NetworkObservatory kernel module
# (Phase 2/M). See docs/phase2-kernel-driver-design.md.
#
# Usage:
#   make KDIR=/path/to/prepared/kernel/tree
# KDIR defaults to the running kernel's /lib/modules/$(uname -r)/build,
# which on WSL2 does not exist by default — see build.sh, which points KDIR
# at a manually-prepared kernel source tree instead (WSL2's kernel isn't a
# packaged distro kernel, so there's no matching linux-headers-* package).

obj-m += netobs.o
netobs-objs := src/netobs_main.o src/netobs_netlink.o src/netobs_collect.o

ccflags-y := -I$(src)/include

KDIR ?= /lib/modules/$(shell uname -r)/build
PWD := $(shell pwd)

all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean

install:
	$(MAKE) -C $(KDIR) M=$(PWD) modules_install
	depmod -a
