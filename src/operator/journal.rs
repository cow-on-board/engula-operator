use crate::{telemetry, Error, Result};
use crate::{api::journal::*};
use chrono::prelude::*;
use futures::{future::BoxFuture, FutureExt, StreamExt};
use k8s_openapi::api::{core::v1::*, apps::v1::*};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::*;
use kube::{
    Error as kubeerror,
    api::{Api, ListParams, Patch, PatchParams, ResourceExt, PostParams},
    client::Client,
    runtime::{
        controller::{Context, Controller, ReconcilerAction},
        events::{Event, EventType, Recorder, Reporter},
    },
    CustomResource, Resource,
};
use prometheus::{
    default_registry, proto::MetricFamily, register_histogram_vec, register_int_counter, HistogramOpts,
    HistogramVec, IntCounter,
};
use serde::Serialize;
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
use tokio::{
    sync::RwLock,
    time::{Duration, Instant},
};
use tracing::{debug, error, event, field, info, instrument, trace, warn, Level, Span};

// Context for our reconciler
#[derive(Clone)]
struct Data {
    /// kubernetes client
    client: Client,
    /// In memory state
    state: Arc<RwLock<State>>,
    /// Various prometheus metrics
    metrics: Metrics,
}

#[instrument(skip(ctx), fields(trace_id))]
async fn reconcile(journal: Journal, ctx: Context<Data>) -> Result<ReconcilerAction, Error> {
    let trace_id = telemetry::get_trace_id();
    Span::current().record("trace_id", &field::display(&trace_id));
    let start = Instant::now();

    let client = ctx.get_ref().client.clone();
    ctx.get_ref().state.write().await.last_event = Utc::now();
    let reporter = ctx.get_ref().state.read().await.reporter.clone();
    let recorder = Recorder::new(client.clone(), reporter, journal.object_ref(&()));
    let name = ResourceExt::name(&journal);
    let ns = ResourceExt::namespace(&journal).expect("journal is namespaced");
    let journals: Api<Journal> = Api::namespaced(client.clone(), &ns);
    let deploys: Api<Deployment> = Api::namespaced(client.clone(), &ns);

    let duration = start.elapsed().as_millis() as f64 / 1000.0;
    ctx.get_ref()
        .metrics
        .reconcile_duration
        .with_label_values(&[])
        .observe(duration);
    ctx.get_ref().metrics.handled_events.inc();
    info!("Reconciled Journal \"{}\" in {}", name, ns);

    let new_status = Patch::Apply(json!({
        "apiVersion": "engula.io/v1alpha1",
        "kind": "Journal",
        "status": JournalStatus {
            deployment_status: None,
        }
    }));
    let ps = PatchParams::apply("cntrlr").force();
    let _o = journals
        .patch_status(&name, &ps, &new_status)
        .await
        .map_err(Error::KubeError)?;
    return match deploys.get(&name).await {
        Ok(current) => update(),
        Err(kube::Error::Api(e)) => create(deploys, journal).await,
        // TODO(gaocegege): Use error_policy here.
        _ => Ok(ReconcilerAction{
            requeue_after: Some(Duration::from_secs(5)),
        })
    };

    // if journal.spec.info.contains("bad") {
    //     recorder
    //         .publish(Event {
    //             type_: EventType::Normal,
    //             reason: "BadJournal".into(),
    //             note: Some(format!("Sending `{}` to detention", name)),
    //             action: "Correcting".into(),
    //             secondary: None,
    //         })
    //         .await
    //         .map_err(Error::KubeError)?;
    // }
}

fn update() -> Result<ReconcilerAction, Error> {
    Ok(
        ReconcilerAction {
            requeue_after: Some(Duration::from_secs(3600 / 2)),
        }
    )
}

async fn create(deploys: Api<Deployment>, journal: Journal) -> Result<ReconcilerAction, Error> {
    let name = ResourceExt::name(&journal);
    let ns = ResourceExt::namespace(&journal).expect("journal is namespaced");
    let deploy = Deployment {
        metadata: ObjectMeta{
            name: Some(name.clone()),
            namespace: Some(ns.clone()),
            labels: Some(BTreeMap::new()),
            annotations: Some(BTreeMap::new()),
            ..Default::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(1),
            selector: LabelSelector {
                match_labels: Some(BTreeMap::new()),
                ..Default::default()
            },
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(BTreeMap::new()),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: name.clone(),
                        image: Some("engula/journal:latest".into()),
                        image_pull_policy: Some("IfNotPresent".into()),
                        command: Some(vec!["journal".into()]),
                        args: Some(vec![name.clone()]),
                        env: Some(vec![
                            EnvVar {
                                name: "JOURNAL_NAME".into(),
                                value: Some(name.clone()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "JOURNAL_NAMESPACE".into(),
                                value: Some(ns.clone()),
                                ..Default::default()
                            },
                        ]),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        ..Default::default()
    };
    let ps = PostParams::default();
    let _o = deploys.create(&ps, &deploy).await.map_err(Error::KubeError)?;
    Ok(
        ReconcilerAction {
            requeue_after: Some(Duration::from_secs(3600 / 2)),
        }
    )
}

fn error_policy(error: &Error, _ctx: Context<Data>) -> ReconcilerAction {
    warn!("reconcile failed: {:?}", error);
    ReconcilerAction {
        requeue_after: Some(Duration::from_secs(360)),
    }
}

/// Metrics exposed on /metrics
#[derive(Clone)]
pub struct Metrics {
    pub handled_events: IntCounter,
    pub reconcile_duration: HistogramVec,
}
impl Metrics {
    fn new() -> Self {
        let reconcile_histogram = register_histogram_vec!(
            "journal_controller_reconcile_duration_seconds",
            "The duration of reconcile to complete in seconds",
            &[],
            vec![0.01, 0.1, 0.25, 0.5, 1., 5., 15., 60.]
        )
        .unwrap();

        Metrics {
            handled_events: register_int_counter!("journal_controller_handled_events", "handled events").unwrap(),
            reconcile_duration: reconcile_histogram,
        }
    }
}

/// In-memory reconciler state exposed on /
#[derive(Clone, Serialize)]
pub struct State {
    #[serde(deserialize_with = "from_ts")]
    pub last_event: DateTime<Utc>,
    #[serde(skip)]
    pub reporter: Reporter,
}
impl State {
    fn new() -> Self {
        State {
            last_event: Utc::now(),
            reporter: "engula-operator".into(),
        }
    }
}

/// Data owned by the Manager
#[derive(Clone)]
pub struct Manager {
    /// In memory state
    state: Arc<RwLock<State>>,
}

/// Example Manager that owns a Controller for Journal
impl Manager {
    /// Lifecycle initialization interface for app
    ///
    /// This returns a `Manager` that drives a `Controller` + a future to be awaited
    /// It is up to `main` to wait for the controller stream.
    pub async fn new() -> (Self, BoxFuture<'static, ()>) {
        let client = Client::try_default().await.expect("create client");
        let metrics = Metrics::new();
        let state = Arc::new(RwLock::new(State::new()));
        let context = Context::new(Data {
            client: client.clone(),
            metrics: metrics.clone(),
            state: state.clone(),
        });

        let journals = Api::<Journal>::all(client);
        // Ensure CRD is installed before loop-watching
        let _r = journals
            .list(&ListParams::default().limit(1))
            .await
            .expect("is the crd installed? please run: cargo run --bin crdgen | kubectl apply -f -");

        // All good. Start controller and return its future.
        let drainer = Controller::new(journals, ListParams::default())
            .run(reconcile, error_policy, context)
            .filter_map(|x| async move { std::result::Result::ok(x) })
            .for_each(|_| futures::future::ready(()))
            .boxed();

        (Self { state }, drainer)
    }

    /// Metrics getter
    pub fn metrics(&self) -> Vec<MetricFamily> {
        default_registry().gather()
    }

    /// State getter
    pub async fn state(&self) -> State {
        self.state.read().await.clone()
    }
}
