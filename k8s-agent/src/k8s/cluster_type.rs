use anyhow::{anyhow, Result};
use base64::prelude::*;
use kube::config::ExecInteractiveMode;
use platz_db::NewK8sCluster;
use std::convert::TryFrom;
use std::fmt;
use tracing::debug;

#[derive(Debug)]
pub enum K8s {
    Eks(aws_sdk_eks::types::Cluster),
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
        }
    }
}

impl From<aws_sdk_eks::types::Cluster> for K8s {
    fn from(cluster: aws_sdk_eks::types::Cluster) -> Self {
        Self::Eks(cluster)
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
        })
    }

    fn server_url(&self) -> Result<&str> {
        Ok(match self {
            K8s::Eks(cluster) => cluster
                .endpoint
                .as_ref()
                .ok_or_else(|| anyhow!("Got empty endpoint"))?,
        })
    }

    fn ca_data(&self) -> Result<&str> {
        Ok(match self {
            K8s::Eks(cluster) => cluster
                .certificate_authority
                .as_ref()
                .ok_or_else(|| anyhow!("No certificate_authority for cluster"))?
                .data
                .as_ref()
                .ok_or_else(|| anyhow!("certificate_authority didn't contain any data"))?,
        })
    }

    fn region(&self) -> Result<aws_arn::Identifier> {
        Ok(match self {
            K8s::Eks(cluster) => {
                let resource_name: aws_arn::ResourceName = cluster
                    .arn
                    .as_ref()
                    .ok_or_else(|| anyhow!("Cluster has no ARN"))?
                    .parse()
                    .map_err(|err| anyhow!("Failed parsing region from ARN: {}", err))?;
                resource_name
                    .region
                    .ok_or_else(|| anyhow!("Cluster ARN has no region"))?
            }
        })
    }

    pub async fn kube_config(&self) -> Result<kube::Config> {
        let kubeconfig = kube::config::Kubeconfig::try_from(self)?;
        let kubeconfig_options = kube::config::KubeConfigOptions {
            context: Some(kubeconfig.contexts.first().unwrap().name.clone()),
            cluster: Some(kubeconfig.clusters.first().unwrap().name.clone()),
            user: Some(kubeconfig.auth_infos.first().unwrap().name.clone()),
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
        let region_name = cluster.region().unwrap().into();
        match cluster {
            K8s::Eks(cluster) => Self {
                provider_id: cluster.arn.as_ref().unwrap().clone(),
                name: cluster.name.as_ref().unwrap().clone(),
                env_id: None,
                region_name,
            },
        }
    }
}

impl TryFrom<&K8s> for kube::config::Kubeconfig {
    type Error = anyhow::Error;

    fn try_from(k8s: &K8s) -> Result<Self, Self::Error> {
        let cluster = k8s.name()?;
        let user = "user";
        let server_url = k8s.server_url()?;
        Ok(Self {
            api_version: Some("v1".to_owned()),
            kind: Some("Config".to_owned()),
            clusters: vec![kube::config::NamedCluster {
                name: cluster.into(),
                cluster: Some(kube::config::Cluster {
                    server: Some(server_url.into()),
                    insecure_skip_tls_verify: Some(false),
                    certificate_authority_data: Some(k8s.ca_data()?.into()),
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
                            k8s.region()?.into(),
                            "--cluster-name".into(),
                            cluster.into(),
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
                    cluster: cluster.into(),
                    user: user.to_owned(),
                    namespace: None,
                    extensions: None,
                }),
            }],
            current_context: Some("default".to_owned()),
            ..Default::default()
        })
    }
}
