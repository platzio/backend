/// The code below was copied from https://github.com/clux/kube-rs/blob/master/examples/pod_attach.rs
use anyhow::{anyhow, Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, AttachedProcess, DeleteParams, ListParams, ResourceExt, WatchEvent};
use log::*;
use std::fmt;

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

pub async fn execute_pod(pods: Api<Pod>, pod: Pod) -> Result<String> {
    let pod_name = pod.metadata.name.as_ref().unwrap();
    pods.create(&Default::default(), &pod)
        .await
        .context("Failed creating pod for running Helm")?;

    let result = wait_for_pod(&pods, pod_name).await;

    debug!("Deleting {}", pod_name);
    pods.delete(pod_name, &DeleteParams::default())
        .await
        .context("Failed deleting Helm pod")?
        .map_left(|pdel| {
            assert_eq!(&ResourceExt::name(&pdel), pod_name);
        });

    match result {
        Ok(exe_result) => Ok(exe_result.output),
        Err(exe_result) => Err(exe_result),
    }
}

async fn wait_for_pod_phase<S, F>(mut stream: S, pred: F) -> Result<()>
where
    S: futures::Stream<Item = kube::Result<WatchEvent<Pod>>> + Unpin,
    F: Fn(&str) -> bool,
{
    while let Some(status) = stream.try_next().await? {
        if let WatchEvent::Modified(pod) = status {
            let status = match pod.status.as_ref() {
                Some(status) => status,
                None => continue,
            };
            match &status.phase {
                Some(phase) => {
                    if pred(phase) {
                        return Ok(());
                    }
                }
                None => continue,
            }
        }
    }
    Err(anyhow!("Failed waiting for pod to reach phase"))
}

async fn wait_for_pod(pods: &Api<Pod>, pod_name: &str) -> Result<PodExecutionResult> {
    let list_params = ListParams::default()
        .fields(&format!("metadata.name={}", pod_name))
        .timeout(5);
    let mut pod_events = tryhard::retry_fn(|| pods.watch(&list_params, "0"))
        .retries(5)
        .await
        .context("Could not start watching for Helm pod status changes")?
        .boxed();

    wait_for_pod_phase(&mut pod_events, |p| p == "Running")
        .await
        .context("Failed waiting for Helm pod to reach Running phase")?;
    info!("Ready to attach to {}", pod_name);

    let attached = pods.attach(pod_name, &Default::default()).await?;
    let output = get_pod_output(attached).await?;

    wait_for_pod_phase(&mut pod_events, |p| p == "Succeeded" || p == "Failed")
        .await
        .context("Failed waiting for Helm pod to reach Succeeded or Failed phase")?;
    info!("Pod {} terminated", pod_name);

    let pod_status = pods.get_status(pod_name).await?;
    let container_status = pod_status
        .status
        .as_ref()
        .unwrap()
        .container_statuses
        .as_ref()
        .unwrap()
        .get(0)
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
                debug!("LINE: {}", line);
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
