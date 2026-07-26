#!/usr/bin/env bash
echo "kernel: $(uname -r)"
echo "--- apt search headers ---"
apt-cache search linux-headers 2>&1 | head -5
echo "--- installed headers ---"
apt list --installed 2>/dev/null | grep -i linux-headers
echo "--- config.gz ---"
if [ -f /proc/config.gz ]; then
  echo "config.gz present"
else
  echo "no config.gz"
fi
