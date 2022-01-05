use std::{collections::{BTreeMap, HashMap}, sync::Arc};
use chrono::{DateTime, Utc};

use futures::stream::StreamExt;
use k8s_openapi::api::{apps::v1::{Deployment, DeploymentSpec}, core::v1::{Container, ContainerPort, ObjectReference, PodSpec, PodTemplateSpec}};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta, OwnerReference};
use kube::{Api, api::{ListParams, PostParams}, client::Client};
use kube::{
    runtime::{
        controller::{Context, Controller, ReconcilerAction},
        events::{Event, EventType, Recorder, Reporter},
    },
};
use kube::Resource;
use kube::ResourceExt;
use tokio::time::Duration;
use tracing::{debug, error, event, field, info, instrument, Level, Span, trace, warn};

use crate::{Error, Result, telemetry};
use crate::api::storage::*;
use crate::operator::journal::{Data, Metrics};

/// Action to be taken upon an `Storage` resource during reconciliation
enum Action {
    /// Create the subresources, this includes spawning `n` pods with Storage service
    Create,
    /// Delete all subresources created in the `Create` phase
    Delete,
    /// This `Storage` resource is in desired state and requires no actions to be taken
    NoOp,
}

#[instrument(skip(ctx), fields(trace_id))]
async fn reconcile(storage: Storage, ctx: Context<Data>) -> Result<ReconcilerAction, Error> {
    let client: Client = ctx.get_ref().client.clone(); // The `Client` is shared -> a clone from the reference is obtained

    // The resource of `Storage` kind is required to have a namespace set. However, it is not guaranteed
    // the resource will have a `namespace` set. Therefore, the `namespace` field on object's metadata
    // is optional and Rust forces the programmer to check for it's existence first.
    let namespace: String = match storage.namespace() {
        None => return Err(Error::MissingObjectKey(".metadata.namespace")),

        // If namespace is known, proceed. In a more advanced version of the operator, perhaps
        // the namespace could be checked for existence first.
        Some(namespace) => namespace,
    };

    // Performs action as decided by the `determine_action` function.
    match determine_action(&storage) {
        Action::Create => {
            // Creates a deployment with `n` Storage service pods

            let name = storage.name(); // Name of the Storage resource is used to name the subresources as well.

            // Invoke creation of a Kubernetes built-in resource named deployment with `n` echo service pods.
            deploy(client, &name, &namespace, &storage).await
        }
        Action::Delete => {
            // Deletes any subresources related to this `Storage` resources. If and only if all subresources
            // are deleted, the finalizer is removed and Kubernetes is free to remove the `Storage` resource.

            Ok(ReconcilerAction {
                requeue_after: None, // Makes no sense to delete after a successful delete, as the resource is gone
            })
        }
        Action::NoOp => Ok(ReconcilerAction {
            // The resource is already in desired state, do nothing and re-check after 30 minutes.
            requeue_after: Some(Duration::from_secs(3600 / 2)),
        }),
    }
}

fn object_to_owner_reference<K: Resource<DynamicType=()>>(
    meta: ObjectMeta,
) -> Result<OwnerReference, Error> {
    Ok(OwnerReference {
        api_version: K::api_version(&()).to_string(),
        kind: K::kind(&()).to_string(),
        name: meta.name.ok_or(Error::MissingObjectKey(".metadata.name"))?,
        uid: meta.uid.ok_or(Error::MissingObjectKey(".metadata.uid"))?,
        ..OwnerReference::default()
    })
}

async fn deploy(
    client: Client,
    name: &str,
    namespace: &str,
    storage: &Storage,
) -> Result<ReconcilerAction, Error> {
    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("app".to_owned(), storage.name().to_owned());

    // Definition of the deployment. Alternatively, a YAML representation could be used as well.
    let deployment: Deployment = Deployment {
        metadata: ObjectMeta {
            name: Some(name.to_owned()),
            namespace: Some(namespace.to_owned()),
            labels: Some(labels.clone()),
            owner_references: Some(vec![OwnerReference {
                controller: Some(true),
                ..object_to_owner_reference::<Storage>(storage.metadata.clone())?
            }]),
            ..Default::default()
        },
        spec: Some(DeploymentSpec {
            selector: LabelSelector {
                match_expressions: None,
                match_labels: Some(labels.clone()),
            },
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: name.to_owned(),
                        image: Some("engula/storage:latest".to_owned()),
                        image_pull_policy: Some("IfNotPresent".to_owned()),
                        command: Some(vec!["storage".to_owned()]),
                        args: Some(vec![name.to_owned()]),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                metadata: Some(ObjectMeta {
                    labels: Some(labels),
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        ..Default::default()
    };

    // Create the deployment defined above
    let ps = PostParams::default();
    let deployment_api: Api<Deployment> = Api::namespaced(client, namespace);
    let _o = deployment_api.create(&ps, &deployment).await.map_err(Error::KubeError)?;
    Ok(
        ReconcilerAction {
            requeue_after: Some(Duration::from_secs(3600 / 2)),
        }
    )
}

/// Resources arrives into reconciliation queue in a certain state. This function looks at
/// the state of given `Storage` resource and decides which actions needs to be performed.
/// The finite set of possible actions is represented by the `Action` enum.
///
/// # Arguments
/// - `echo`: A reference to `Storage` being reconciled to decide next action upon.
fn determine_action(storage: &Storage) -> Action {
    return if storage.meta().deletion_timestamp.is_some() {
        Action::Delete
    } else if storage
        .meta()
        .finalizers
        .as_ref()
        .map_or(true, |finalizers| finalizers.is_empty())
    {
        Action::Create
    } else {
        Action::NoOp
    };
}
