# Design Document

## Background

[engula][] is a serverless storage engine that empowers engineers to build reliable and cost-effective databases. But it is hard to deploy it to Kubernetes since the dependency order of [engula][] is complex. 

This project is to provide a declarative way to manage [engula][] clusters on Kubernetes easily. 

Please refer to [engula/engula#214](https://github.com/engula/engula/discussions/214) for more details.

## Design

### CRD API

#### Journal

```YAML
apiVersion: engula.io/v1alpha1
kind: Journal
metadata:
  name: sample
  labels:
    app: engula
spec:
  template:
    metadata:
      labels:
        app: engula
    spec:
      containers:
      - args:
        - journal
        - start
        command:
        - /bin/engula
        image: uhub.service.ucloud.cn/engula/engula:0.3
        name: engula-journal-serivce
```

#### Storage

Similar to Journal

#### Kernel

```YAML
apiVersion: engula.io/v1alpha1
kind: Kernel
metadata:
  name: sample
  labels:
    app: engula
spec:
  journal:
    namespace: default
    name: sample
  storage:
    namespace: default
    name: sample
  template:
    metadata:
      labels:
        app: engula
    spec:
      containers:
      - args:
        - kernel
        - start
        command:
        - /bin/engula
        image: uhub.service.ucloud.cn/engula/engula:0.3
        name: engula-kernel-serivce
```

## Roadmap

### v0.0.1

- Support in-memory storage and journal

### v0.0.2

- Support local disk

[engula]: https://github.com/engula/engula
