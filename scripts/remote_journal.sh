#!/bin/bash
# Stream journalctl logs from a remote server via SSH and pipe to lazylog

set -e

ip_address="$1"

if [ -z "$ip_address" ]; then
  read -p "Enter the IP address: " ip_address
fi

# -n      Redirects stdin from /dev/null (actually, prevents reading from stdin).  This must be used when ssh is run in the background.
ssh -n root@"$ip_address" "journalctl -f" | lazylog
