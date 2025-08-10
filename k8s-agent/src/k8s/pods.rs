/// The code below was copied from https://github.com/clux/kube-rs/blob/master/examples/pod_attach.rs
use crate::utils::create_interval_stream;
use anyhow::{anyhow, Context, Result};
use futures::{stream, StreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, AttachedProcess, DeleteParams, PostParams, ResourceExt};
use kube::runtime::watcher::{self, Event};
use std::{fmt, time::Duration};
use tap::TapFallible;
use tokio::select;
use tracing::{debug, error};

#[derive(Debug, thiserror::Error)]
pub struct PodExecutionResult {
    exit_code: i32,
    output: String,
}

impl fmt::Display for PodExecutionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.exit_code == 0 {
            write!(f, "{}", self.output)
        } else {
            write!(f, "Execution failed ({}): {}", self.exit_code, self.output)
        }
    }
}

async fn client(namespace: &str) -> Result<Api<Pod>> {
    let client = kube::Client::try_default()
        .await
        .context("Failed initializing kube client")?;
    Ok(Api::namespaced(client, namespace))
}

async fn create_pod(namespace: &str, create_params: &PostParams, pod: &Pod) -> Result<()> {
    let client = client(namespace).await?;
    client
        .create(create_params, pod)
        .await
        .context("pods.create failed")
        .map(drop)
}

async fn delete_pod(
    namespace: &str,
    pod_name: &str,
    delete_params: &DeleteParams,
) -> Result<either::Either<Pod, kube::core::Status>> {
    let client: Api<Pod> = client(namespace).await?;
    client
        .delete(pod_name, delete_params)
        .await
        .context("pods.delete failed")
}

#[tracing::instrument(err, skip_all)]
pub async fn execute_pod(namespace: &str, pod: Pod) -> Result<String> {
    let pod_name = pod.metadata.name.clone().unwrap();

    let create_params = Default::default();
    debug!("Creating pod...");
    tryhard::retry_fn(|| create_pod(namespace, &create_params, &pod))
        .retries(10)
        .fixed_backoff(Duration::from_millis(500))
        .await
        .context("Failed creating pod for running Helm")?;

    debug!("Pod created");

    let result = wait_for_pod(&client(namespace).await?, &pod_name).await;

    debug!("Deleting pod...");
    let delete_params = Default::default();
    tryhard::retry_fn(|| delete_pod(namespace, &pod_name, &delete_params))
        .retries(10)
        .fixed_backoff(Duration::from_millis(500))
        .await
        .context("Failed deleting Helm pod")?
        .map_left(|pdel| {
            assert_eq!(pdel.name_any(), pod_name);
        });

    debug!("Pod deleted");

    result
        .map(|exe_result| {
            debug!("{pod_name} deletion succeed");
            exe_result.output
        })
        .tap_err(|e| error!("{pod_name} deletion failed: {e:?}"))
}

async fn wait_for_pod_phase<S, F>(
    mut stream: S,
    pred: F,
    timeout_duration: Duration,
) -> Result<String>
where
    S: futures::Stream<
            Item = Result<kube::runtime::watcher::Event<Pod>, kube::runtime::watcher::Error>,
        > + Unpin,
    F: Fn(&str) -> bool,
{
    let mut logs_timer_stream = create_interval_stream(Duration::from_secs(60));
    let timeout_sleep = tokio::time::sleep(timeout_duration);
    tokio::pin!(timeout_sleep);

    loop {
        select! {
            biased;
            Some(event) = stream.next() => {
                match event {
                    Ok(Event::Apply(pod) | Event::InitApply(pod)) => {
                        let status = match pod.status.as_ref() {
                            Some(status) => status,
                            None => continue,
                        };
                        match &status.phase {
                            Some(phase) => {
                                if pred(phase) {
                                    debug!("Reached {phase} phase");
                                    return Ok(phase.clone());
                                }
                            }
                            None => continue,
                        }
                    }
                    Ok(Event::Init | Event::InitDone | Event::Delete(_)) => {}
                    Err(e) => debug!("Recovering from watcher error: {e:?}"),
                }
            },
            () = &mut timeout_sleep => {
                debug!("Failed waiting for pod to reach phase");
                return Err(anyhow!("Failed waiting for pod to reach phase"))
            },
            _ = logs_timer_stream.next() => {
                debug!("Still waiting for pod phase");
            }
        }
    }
}

#[tracing::instrument(err, skip(pods))]
async fn wait_for_pod(pods: &Api<Pod>, pod_name: &str) -> Result<PodExecutionResult> {
    let watcher_config = watcher::Config::default()
        .fields(&format!("metadata.name={pod_name}"))
        .timeout(5);

    let mut pod_events = watcher::watcher(pods.clone(), watcher_config).boxed();
    let is_pod_finished = |phase_name: &str| -> bool {
        let result = phase_name.eq_ignore_ascii_case("Succeeded")
            || phase_name.eq_ignore_ascii_case("Failed");

        tracing::debug!(phase_name, result, "is_pod_finished");
        result
    };

    debug!("Waiting for pod to be ready");
    let mut pod_phase = wait_for_pod_phase(
        &mut pod_events,
        |p| {
            let result = !p.eq_ignore_ascii_case("Pending") && !p.eq_ignore_ascii_case("Unknown");
            tracing::debug!(phase_name = p, result, "pod started?");
            result
        },
        Duration::from_secs(60),
    )
    .await
    .with_context(|| format!("Failed waiting for Helm pod {pod_name} to start running"))?;
    debug!("Attaching to {pod_name} (phase: {pod_phase})");
    let attached = pods.attach(pod_name, &Default::default()).await?;
    let output = get_pod_output(attached)
        .await
        .unwrap_or_else(|_| "<Output N/A>".to_string());

    debug!("Waiting for pod to finish");
    pod_phase = wait_for_pod_phase(
        &mut pod_events,
        is_pod_finished,
        Duration::from_secs(60 * 10),
    )
    .await
    .with_context(|| {
        format!("Failed waiting for Helm pod {pod_name} to reach Succeeded or Failed phase",)
    })?;
    tracing::debug!(phase = pod_phase, "Terminated");

    let pod_status = pods.get_status(pod_name).await?;
    let container_status = pod_status
        .status
        .as_ref()
        .unwrap()
        .container_statuses
        .as_ref()
        .unwrap()
        .first()
        .unwrap();
    let container_state = container_status
        .state
        .as_ref()
        .unwrap()
        .terminated
        .as_ref()
        .unwrap();
    let exit_code = container_state.exit_code;

    debug!("Pod {} exited with {}", pod_name, exit_code);

    let result = PodExecutionResult { exit_code, output };
    if exit_code == 0 {
        Ok(result)
    } else {
        Err(result.into())
    }
}

async fn get_pod_output(mut attached: AttachedProcess) -> Result<String> {
    debug!("Getting stdout/stderr");

    let stdout = tokio_util::io::ReaderStream::new(attached.stdout().unwrap());
    let stderr = tokio_util::io::ReaderStream::new(attached.stderr().unwrap());

    let lines: Vec<String> = stream::select(stdout, stderr)
        .map(|res| match res {
            Ok(bytes) => {
                let line = String::from_utf8_lossy(&bytes).to_string();
                tracing::debug!(line);
                line
            }
            Err(_) => String::new(),
        })
        .collect()
        .await;

    debug!("Waiting for process to finish");
    let join_lines = attached.join().await.map_or_else(
        |err| format!("An error has occurred while waiting for Helm pod to finish: {err:?}\n\n",),
        |_| Default::default(),
    );

    Ok(join_lines + &lines.join(""))
}
