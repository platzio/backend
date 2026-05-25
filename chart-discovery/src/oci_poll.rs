use crate::charts::{HELM_ARTIFACT_MEDIA_TYPE, download_chart_via_oci, record_helm_chart};
use crate::kind::get_or_create_kind;
use anyhow::{Result, anyhow};
use chrono::prelude::*;
use clap::Parser;
use platz_chart_ext::ChartExt;
use platz_db::schema::helm_chart::HelmChart;
use platz_db::schema::helm_registry::{HelmRegistry, HelmRegistryProvider, NewHelmRegistry};
use serde::Deserialize;
use std::collections::HashSet;
use tokio::time;
use tracing::{debug, info, warn};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Parser)]
#[group(skip)]
pub struct Config {
    /// Base URL of the OCI registry to poll, e.g. `http://localhost:5001`.
    /// Required when `PLATZ_REGISTRY_PROVIDER=oci`.
    #[clap(long, env = "PLATZ_OCI_REGISTRY_URL")]
    pub oci_registry_url: Option<Url>,

    /// How often to poll the registry's `_catalog` and tag lists for new charts.
    #[clap(long, env = "PLATZ_OCI_POLL_INTERVAL", default_value = "5s")]
    pub oci_poll_interval: humantime::Duration,
}

pub async fn run(config: &Config) -> Result<()> {
    let url = config
        .oci_registry_url
        .clone()
        .ok_or_else(|| anyhow!("PLATZ_OCI_REGISTRY_URL is required when provider=oci"))?;
    let domain = registry_domain(&url)?;
    let client = reqwest::Client::builder().build()?;

    info!("Polling OCI registry at {url}");

    let mut interval = time::interval(*config.oci_poll_interval);
    let mut seen_pairs: HashSet<(String, String)> = HashSet::new();

    loop {
        interval.tick().await;
        if let Err(err) = poll_once(&client, &url, &domain, &mut seen_pairs).await {
            warn!("OCI poll iteration failed: {err:?}");
        }
    }
}

async fn poll_once(
    client: &reqwest::Client,
    url: &Url,
    domain: &str,
    seen_pairs: &mut HashSet<(String, String)>,
) -> Result<()> {
    let catalog = list_repos(client, url).await?;
    debug!("Catalog: {} repos", catalog.len());

    for repo in catalog {
        let tags = match list_tags(client, url, &repo).await {
            Ok(tags) => tags,
            Err(err) => {
                warn!("Failed listing tags for {repo}: {err:?}");
                continue;
            }
        };

        for tag in tags {
            let key = (repo.clone(), tag.clone());
            if seen_pairs.contains(&key) {
                continue;
            }
            match handle_tag(client, url, domain, &repo, &tag).await {
                Ok(()) => {
                    seen_pairs.insert(key);
                }
                Err(err) => {
                    warn!("Failed handling {repo}:{tag}: {err:?}");
                }
            }
        }
    }

    Ok(())
}

async fn handle_tag(
    client: &reqwest::Client,
    url: &Url,
    domain: &str,
    repo: &str,
    tag: &str,
) -> Result<()> {
    let manifest = fetch_manifest(client, url, repo, tag).await?;

    if manifest.body.config.media_type != HELM_ARTIFACT_MEDIA_TYPE {
        debug!(
            "Skipping {repo}:{tag} (media type {})",
            manifest.body.config.media_type
        );
        return Ok(());
    }

    // The Distribution API doesn't expose pushed-at, so we synthesise it from
    // "now" the first time we observe the tag. Chart.yaml metadata still wins
    // for any version/branch/commit info.
    let created_at = Utc::now();

    let registry = ensure_registry(domain, repo).await?;

    if HelmChart::find_by_registry_and_digest(registry.id, manifest.digest.clone())
        .await?
        .is_some()
    {
        debug!("Chart {repo}:{tag} already in DB, skipping");
        return Ok(());
    }

    let chart_ext = match download_chart_via_oci(domain, repo, tag).await {
        Ok(path) => ChartExt::from_path(&path).await?,
        Err(err) => ChartExt::new_with_error(err.to_string()),
    };

    record_helm_chart(
        registry.id,
        manifest.digest,
        tag.to_owned(),
        created_at,
        chart_ext,
    )
    .await?;

    Ok(())
}

async fn ensure_registry(domain: &str, repo: &str) -> Result<HelmRegistry> {
    if let Some(reg) =
        HelmRegistry::find_by_domain_and_repo(domain.to_owned(), repo.to_owned()).await?
    {
        return Ok(reg);
    }
    let new = NewHelmRegistry {
        created_at: Utc::now(),
        domain_name: domain.to_owned(),
        repo_name: repo.to_owned(),
        kind_id: get_or_create_kind(repo).await?.id,
        provider: HelmRegistryProvider::Oci,
    };
    info!("Saving new OCI helm registry {:?}", new);
    Ok(new.insert().await?)
}

#[derive(Debug, Deserialize)]
struct CatalogResponse {
    repositories: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    tags: Option<Vec<String>>,
}

#[derive(Debug)]
struct ManifestFetchResult {
    digest: String,
    body: ManifestBody,
}

#[derive(Debug, Deserialize)]
struct ManifestBody {
    config: ManifestConfig,
}

#[derive(Debug, Deserialize)]
struct ManifestConfig {
    #[serde(rename = "mediaType")]
    media_type: String,
}

async fn list_repos(client: &reqwest::Client, base: &Url) -> Result<Vec<String>> {
    let url = base.join("v2/_catalog")?;
    let resp: CatalogResponse = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(resp.repositories)
}

async fn list_tags(client: &reqwest::Client, base: &Url, repo: &str) -> Result<Vec<String>> {
    let url = base.join(&format!("v2/{repo}/tags/list"))?;
    let resp: TagsResponse = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(resp.tags.unwrap_or_default())
}

async fn fetch_manifest(
    client: &reqwest::Client,
    base: &Url,
    repo: &str,
    tag: &str,
) -> Result<ManifestFetchResult> {
    let url = base.join(&format!("v2/{repo}/manifests/{tag}"))?;
    let resp = client
        .get(url)
        .header(
            "Accept",
            "application/vnd.oci.image.manifest.v1+json, \
             application/vnd.docker.distribution.manifest.v2+json",
        )
        .send()
        .await?
        .error_for_status()?;

    let digest = resp
        .headers()
        .get("Docker-Content-Digest")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| {
            // Fallback: synthesise a non-colliding key from repo+tag if the registry
            // doesn't return the digest header (some bare-bones implementations).
            format!("oci-pseudo:{repo}:{tag}:{}", Uuid::new_v4())
        });

    let body: ManifestBody = resp.json().await?;
    Ok(ManifestFetchResult { digest, body })
}

fn registry_domain(url: &Url) -> Result<String> {
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("OCI registry URL has no host: {url}"))?;
    let domain = match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    };
    Ok(domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_domain_includes_port() {
        let url = Url::parse("http://localhost:5001").unwrap();
        assert_eq!(registry_domain(&url).unwrap(), "localhost:5001");
    }

    #[test]
    fn registry_domain_omits_default_port() {
        let url = Url::parse("https://example.com").unwrap();
        assert_eq!(registry_domain(&url).unwrap(), "example.com");
    }

    #[test]
    fn registry_domain_with_explicit_port() {
        let url = Url::parse("https://oci.example.com:443/").unwrap();
        // url crate normalises the default port for the scheme away.
        assert_eq!(registry_domain(&url).unwrap(), "oci.example.com");
    }
}
