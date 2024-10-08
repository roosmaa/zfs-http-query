#!/bin/bash

case "$1" in
  list) ;;
  *)
    echo "usage: $0 list [args]"
    exit 1
    ;;
esac

cmd="$1"
shift
args="${*@Q}"

exec curl --unix-socket "/var/run/zfs-http-query/zfs-http-query.sock" \
  -XPOST "http://zfs-http-query/zpool/$cmd" \
  -H "Content-Type: text/plain" \
  -d "$args"
