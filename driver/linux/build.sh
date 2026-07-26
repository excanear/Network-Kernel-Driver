#!/usr/bin/env bash
# Builds the NetworkObservatory kernel module against a prepared kernel
# source tree (see docs/phase2-kernel-driver-design.md / driver/linux README).
#
# WSL2's kernel is a custom Microsoft build, not a packaged distro kernel, so
# there's no `linux-headers-$(uname -r)` package to install. Instead:
#   1. Clone https://github.com/microsoft/WSL2-Linux-Kernel at the tag
#      matching `uname -r` exactly (e.g. linux-msft-wsl-6.18.33.2).
#   2. Seed .config from the running kernel: zcat /proc/config.gz > .config
#   3. make olddefconfig && make modules_prepare
#   4. Point KDIR at that tree when building this module.
set -euo pipefail

KDIR="${1:-$HOME/wsl2-kernel}"
if [ ! -f "$KDIR/Module.symvers" ]; then
  echo "error: $KDIR doesn't look like a prepared kernel tree (no Module.symvers)." >&2
  echo "Run 'make modules_prepare' there first — see the comment at the top of this script." >&2
  exit 1
fi

cd "$(dirname "${BASH_SOURCE[0]}")"
make KDIR="$KDIR"
echo "Built: $(pwd)/netobs.ko"
