# zfs-http-query

> [!WARNING]
> This is homelab quality software, and not meant for production usage. You have been warned.

ZFS HTTP query is a simple unix-socket based HTTP API primarily meant to give [Netdata](https://www.netdata.cloud/) monitoring access to [zpool](https://openzfs.github.io/openzfs-docs/man/master/8/zpool.8.html) utility on a [Talos](https://www.talos.dev/) host node.

It works by spawning a daemon process on the node and exposing the Unix socket under a well-known path. Unprivileged workloads can then mount that path to gain access to the underlying zpool functionality. 

## Set-up

First, you want to deploy it as a DaemonSet on your ZFS enabled Talos nodes. Something like this will do:

```yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  labels:
    name: zfs-http-query
  name: zfs-http-query
spec:
  selector:
    matchLabels:
      name: zfs-http-query
  template:
    metadata:
      labels:
        name: zfs-http-query
    spec:
      containers:
        - name: zfs-http-query
          image: ghcr.io/roosmaa/zfs-http-query
          env:
            - name: PATH
              value: /usr/local/sbin
          volumeMounts:
            - name: opt
              mountPath: /opt/zfs-http-query
            - name: run
              mountPath: /var/run/zfs-http-query
            - name: lib
              mountPath: /lib
              readOnly: true
            - name: usr-lib
              mountPath: /usr/lib
              readOnly: true
            - name: usr-local-lib
              mountPath: /usr/local/lib
              readOnly: true
            - name: usr-local-sbin-zpool
              mountPath: /usr/local/sbin/zpool
              readOnly: true
          securityContext:
            privileged: true
      volumes:
        - name: opt
          hostPath:
            path: /opt/zfs-http-query
            type: DirectoryOrCreate
        - name: run
          hostPath:
            path: /var/run/zfs-http-query
            type: DirectoryOrCreate
        - name: lib
          hostPath:
            path: /lib
            type: Directory
        - name: usr-lib
          hostPath:
            path: /usr/lib
            type: Directory
        - name: usr-local-lib
          hostPath:
            path: /usr/local/lib
            type: Directory
        - name: usr-local-sbin-zpool
          hostPath:
            path: /usr/local/sbin/zpool
  updateStrategy:
    type: RollingUpdate
```

Now, in your Netdata Helm values.yaml you can make use of this like so:

```yaml
netdata:
  # ...
  child:
    configs:
      zfspool:
        enabled: true
        path: /etc/netdata/go.d/zfspool.conf
        data: |
          jobs:
            - name: zfspool
              binary_path: /opt/zfs-http-query/bin/zpool
    extraVolumeMounts:
      - name: opt-zfs-http-query
        mountPath: /opt/zfs-http-query
        readOnly: true
      - name: run-zfs-http-query
        mountPath: /var/run/zfs-http-query
        readOnly: true
    extraVolumes:
      - name: opt-zfs-http-query
        hostPath:
          path: /opt/zfs-http-query
          type: DirectoryOrCreate
      - name: run-zfs-http-query
        hostPath:
          path: /var/run/zfs-http-query
          type: DirectoryOrCreate
```
