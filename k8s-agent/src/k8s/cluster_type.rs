use anyhow::{Result, anyhow};
use base64::prelude::*;
use kube::config::ExecInteractiveMode;
use platz_db::schema::k8s_cluster::NewK8sCluster;
use std::convert::TryFrom;
use std::fmt;
use tracing::debug;

const LOCAL_REGION: &str = "local";

#[derive(Debug)]
pub enum K8s {
    Eks(Box<aws_sdk_eks::types::Cluster>),
    Local(Box<LocalCluster>),
}

#[derive(Debug, Clone)]
pub struct LocalCluster {
    /// Display name for the cluster, derived from the kubeconfig context name.
    pub name: String,
    /// Synthetic provider id (e.g. `local:platz-local`) used as a stable key in
    /// the `k8s_clusters.provider_id` column. Avoids colliding with EKS ARNs.
    pub provider_id: String,
    /// The kubeconfig that drives both the in-process kube client and the
    /// helm pod's `KUBECONFIG_BASE64`.
    pub kubeconfig: kube::config::Kubeconfig,
}

impl fmt::Display for K8s {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eks(cluster) => write!(
                f,
                "EKS({})",
                cluster
                    .endpoint
                    .as_ref()
                    .unwrap_or(&String::from("unknown"))
            ),
            Self::Local(cluster) => write!(f, "Local({})", cluster.name),
        }
    }
}

impl From<aws_sdk_eks::types::Cluster> for K8s {
    fn from(cluster: aws_sdk_eks::types::Cluster) -> Self {
        Self::Eks(Box::new(cluster))
    }
}

impl From<LocalCluster> for K8s {
    fn from(cluster: LocalCluster) -> Self {
        Self::Local(Box::new(cluster))
    }
}

impl K8s {
    pub async fn kube_client(&self) -> Result<kube::Client> {
        Ok(kube::Client::try_from(self.kube_config().await?)?)
    }

    pub fn name(&self) -> Result<&str> {
        Ok(match self {
            K8s::Eks(cluster) => cluster
                .name
                .as_ref()
                .ok_or_else(|| anyhow!("Cluster has empty name"))?,
            K8s::Local(cluster) => &cluster.name,
        })
    }

    fn provider_id(&self) -> Result<String> {
        Ok(match self {
            K8s::Eks(cluster) => cluster
                .arn
                .as_ref()
                .ok_or_else(|| anyhow!("Cluster has no ARN"))?
                .clone(),
            K8s::Local(cluster) => cluster.provider_id.clone(),
        })
    }

    fn region_name(&self) -> Result<String> {
        Ok(match self {
            K8s::Eks(_) => self.eks_region()?.into(),
            K8s::Local(_) => LOCAL_REGION.to_owned(),
        })
    }

    fn eks_region(&self) -> Result<aws_arn::Identifier> {
        match self {
            K8s::Eks(cluster) => {
                let resource_name: aws_arn::ResourceName = cluster
                    .arn
                    .as_ref()
                    .ok_or_else(|| anyhow!("Cluster has no ARN"))?
                    .parse()
                    .map_err(|err| anyhow!("Failed parsing region from ARN: {}", err))?;
                resource_name
                    .region
                    .ok_or_else(|| anyhow!("Cluster ARN has no region"))
            }
            K8s::Local(_) => Err(anyhow!("Local clusters have no AWS region")),
        }
    }

    pub async fn kube_config(&self) -> Result<kube::Config> {
        let kubeconfig = kube::config::Kubeconfig::try_from(self)?;
        let context = kubeconfig
            .current_context
            .clone()
            .or_else(|| kubeconfig.contexts.first().map(|c| c.name.clone()))
            .ok_or_else(|| anyhow!("Kubeconfig has no contexts"))?;
        let cluster = kubeconfig
            .clusters
            .first()
            .ok_or_else(|| anyhow!("Kubeconfig has no clusters"))?
            .name
            .clone();
        let user = kubeconfig
            .auth_infos
            .first()
            .ok_or_else(|| anyhow!("Kubeconfig has no auth infos"))?
            .name
            .clone();
        let kubeconfig_options = kube::config::KubeConfigOptions {
            context: Some(context),
            cluster: Some(cluster),
            user: Some(user),
        };
        Ok(kube::Config::from_custom_kubeconfig(kubeconfig, &kubeconfig_options).await?)
    }

    pub fn base64_kubeconfig(&self) -> Result<String> {
        let kubeconfig = kube::config::Kubeconfig::try_from(self)?;
        let yaml_kubeconfig = serde_yaml::to_string(&kubeconfig)?;
        debug!("Generated yaml kubeconfig:\n{}", yaml_kubeconfig);
        Ok(BASE64_STANDARD.encode(yaml_kubeconfig))
    }
}

impl From<&K8s> for NewK8sCluster {
    fn from(cluster: &K8s) -> Self {
        Self {
            provider_id: cluster.provider_id().unwrap(),
            name: cluster.name().unwrap().to_owned(),
            env_id: None,
            region_name: cluster.region_name().unwrap(),
        }
    }
}

impl TryFrom<&K8s> for kube::config::Kubeconfig {
    type Error = anyhow::Error;

    fn try_from(k8s: &K8s) -> Result<Self, Self::Error> {
        match k8s {
            K8s::Eks(cluster) => eks_kubeconfig(cluster),
            K8s::Local(cluster) => Ok(cluster.kubeconfig.clone()),
        }
    }
}

fn eks_kubeconfig(cluster: &aws_sdk_eks::types::Cluster) -> Result<kube::config::Kubeconfig> {
    let cluster_name = cluster
        .name
        .as_ref()
        .ok_or_else(|| anyhow!("Cluster has empty name"))?;
    let server_url = cluster
        .endpoint
        .as_ref()
        .ok_or_else(|| anyhow!("Got empty endpoint"))?;
    let ca_data = cluster
        .certificate_authority
        .as_ref()
        .ok_or_else(|| anyhow!("No certificate_authority for cluster"))?
        .data
        .as_ref()
        .ok_or_else(|| anyhow!("certificate_authority didn't contain any data"))?;
    let region: String = {
        let resource_name: aws_arn::ResourceName = cluster
            .arn
            .as_ref()
            .ok_or_else(|| anyhow!("Cluster has no ARN"))?
            .parse()
            .map_err(|err| anyhow!("Failed parsing region from ARN: {}", err))?;
        resource_name
            .region
            .ok_or_else(|| anyhow!("Cluster ARN has no region"))?
            .into()
    };
    let user = "user";
    Ok(kube::config::Kubeconfig {
        api_version: Some("v1".to_owned()),
        kind: Some("Config".to_owned()),
        clusters: vec![kube::config::NamedCluster {
            name: cluster_name.clone(),
            cluster: Some(kube::config::Cluster {
                server: Some(server_url.clone()),
                insecure_skip_tls_verify: Some(false),
                certificate_authority_data: Some(ca_data.clone()),
                ..Default::default()
            }),
        }],
        auth_infos: vec![kube::config::NamedAuthInfo {
            name: user.to_owned(),
            auth_info: Some(kube::config::AuthInfo {
                exec: Some(kube::config::ExecConfig {
                    command: Some("aws".into()),
                    args: Some(vec![
                        "eks".into(),
                        "get-token".into(),
                        "--region".into(),
                        region,
                        "--cluster-name".into(),
                        cluster_name.clone(),
                    ]),
                    api_version: Some("client.authentication.k8s.io/v1".to_owned()),
                    interactive_mode: Some(ExecInteractiveMode::Never),
                    env: None,
                    drop_env: None,
                    provide_cluster_info: false,
                    cluster: None,
                }),
                ..Default::default()
            }),
        }],
        contexts: vec![kube::config::NamedContext {
            name: "default".to_owned(),
            context: Some(kube::config::Context {
                cluster: cluster_name.clone(),
                user: Some(user.to_owned()),
                namespace: None,
                extensions: None,
            }),
        }],
        current_context: Some("default".to_owned()),
        ..Default::default()
    })
}
