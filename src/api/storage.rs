use kube::{
    api::{Api, ListParams, Patch, PatchParams, ResourceExt},
    client::Client,
    runtime::{
        controller::{Context, Controller, ReconcilerAction},
        events::{Event, EventType, Recorder, Reporter},
    },
    CustomResource, Resource,
};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use k8s_openapi::api::{core::v1 as corev1, apps::v1 as appsv1};

/// Our Storage custom resource spec
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(kind = "Storage", group = "engula.io", version = "v1alpha1", namespaced)]
#[kube(status = "StorageStatus")]
pub struct StorageSpec {
    pub template: Option<corev1::PodTemplateSpec>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct StorageStatus {
    pub deployment_status: Option<appsv1::DeploymentStatus>,
}
