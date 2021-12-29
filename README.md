# Engula Operator

The [engula][] operator manages [engula][] clusters deployed to Kubernetes and automates tasks related to operating an [engula][] cluster.

## Background

[engula][] is a serverless storage engine that empowers engineers to build reliable and cost-effective databases. But it is hard to deploy it to Kubernetes since the dependency order of [engula][] is complex. 

This project is to provide a declarative way to manage [engula][] clusters on Kubernetes easily. 

Please refer to [engula/engula#214](https://github.com/engula/engula/discussions/214) for more details.

## Benefits to the TiDB Community

The TiDB server has a distributed architecture with flexible and elastic scalability. This architecture supports pluggable storage drivers and engines, which powers you to customize your database solutions based on your own business requirements. 

Engula is one of the potential storage engines that can serve data read/write requests from the TiDB servers. Engula is designed to be elastic, adaptive and extensible. The combination offers a totally new experience in addition to the many new features.

There is a TiDB operator to deploy TiDB clusters on Kubernetes, which uses TiKV as the default storage layer. The [engula-operator][] is proposed to bridge the gap between TiDB and Engula.

## Design

[kube-rs] will be used to manage 6 CRDs:

- Cluster
- Journal
- Storage
- Background
- KernelService
- Engine

`Cluster` is the high-level CRD. End users just need to create or update this CRD in most cases. The `Cluster` owns the underlying CRDs like `Journal` and so on.

[kube-rs]: https://github.com/kube-rs/kube-rs
[engula]: https://github.com/engula/engula
