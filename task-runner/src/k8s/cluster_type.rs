use anyhow::{anyhow, Result};
use log::*;
use platz_db::NewK8sCluster;
use rusoto_core::Region;
use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub enum K8s {
    Eks(rusoto_eks::Cluster),
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

impl From<rusoto_eks::Cluster> for K8s {
    fn from(cluster: rusoto_eks::Cluster) -> Self {
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

    fn region(&self) -> Result<Region> {
        Ok(match self {
            K8s::Eks(cluster) => {
                let arn: aws_arn::ARN = cluster
                    .arn
                    .as_ref()
                    .ok_or_else(|| anyhow!("Cluster has no arn"))?
                    .parse()
                    .unwrap();
                Region::from_str(
                    &arn.region
                        .ok_or_else(|| anyhow!("Cluster arn has no region"))?,
                )?
            }
        })
    }

    pub async fn kube_config(&self) -> Result<kube::Config> {
        let kubeconfig = kube::config::Kubeconfig::try_from(self)?;
        let kubeconfig_options = kube::config::KubeConfigOptions {
            context: Some(kubeconfig.contexts.get(0).unwrap().name.clone()),
            cluster: Some(kubeconfig.clusters.get(0).unwrap().name.clone()),
            user: Some(kubeconfig.auth_infos.get(0).unwrap().name.clone()),
        };
        Ok(kube::Config::from_custom_kubeconfig(kubeconfig, &kubeconfig_options).await?)
    }

    pub fn base64_kubeconfig(&self) -> Result<String> {
        let kubeconfig = kube::config::Kubeconfig::try_from(self)?;
        let yaml_kubeconfig = serde_yaml::to_string(&kubeconfig)?;
        warn!("Generated yaml kubeconfig:\n{}", yaml_kubeconfig);
        Ok(base64::encode(yaml_kubeconfig))
    }
}

impl From<&K8s> for NewK8sCluster {
    fn from(cluster: &K8s) -> Self {
        let region_name = cluster.region().unwrap().name().to_owned();
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
                cluster: kube::config::Cluster {
                    server: server_url.into(),
                    insecure_skip_tls_verify: Some(false),
                    certificate_authority: None,
                    certificate_authority_data: Some(k8s.ca_data()?.into()),
                    extensions: None,
                    proxy_url: None,
                },
            }],
            auth_infos: vec![kube::config::NamedAuthInfo {
                name: user.to_owned(),
                auth_info: kube::config::AuthInfo {
                    exec: Some(kube::config::ExecConfig {
                        command: "aws".into(),
                        args: Some(vec![
                            "eks".into(),
                            "get-token".into(),
                            "--region".into(),
                            k8s.region()?.name().into(),
                            "--cluster".into(),
                            cluster.into(),
                        ]),
                        api_version: Some("client.authentication.k8s.io/v1alpha1".to_owned()),
                        env: None,
                    }),
                    ..Default::default()
                },
            }],
            contexts: vec![kube::config::NamedContext {
                name: "default".to_owned(),
                context: kube::config::Context {
                    cluster: cluster.into(),
                    user: user.to_owned(),
                    namespace: None,
                    extensions: None,
                },
            }],
            current_context: Some("default".to_owned()),
            ..Default::default()
        })
    }
}
