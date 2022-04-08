use anyhow::{anyhow, Result};
use chrono::prelude::*;
use itertools::Itertools;
use log::*;
use platz_db::{HelmRegistry, NewHelmRegistry};
use rusoto_ecr::{DescribeRepositoriesRequest, Ecr, EcrClient, Repository};
use rusoto_utils::creds::rusoto_client;
use rusoto_utils::regions::Region;

pub async fn find_and_save_ecr_repo(region: Region, repo_name: &str) -> Result<HelmRegistry> {
    let client = rusoto_client(env!("CARGO_CRATE_NAME").to_owned())?;
    let ecr = EcrClient::new_with_client(client, region);
    let res = ecr
        .describe_repositories(DescribeRepositoriesRequest {
            repository_names: Some(vec![repo_name.into()]),
            ..Default::default()
        })
        .await?;

    let repo = match res.repositories {
        None => return Err(anyhow!(format!("Could not find ECR repo: {}", repo_name))),
        Some(repositories) => repositories
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("ECR repo not found: {}", repo_name))?,
    };

    save_ecr_repo_in_db(repo).await
}

async fn save_ecr_repo_in_db(repo: Repository) -> Result<HelmRegistry> {
    let created_at = match repo.created_at {
        None => {
            return Err(anyhow!(format!(
                "ECR repository has no created_at: {:?}",
                repo
            )))
        }
        Some(secs) => DateTime::from_utc(NaiveDateTime::from_timestamp(secs as i64, 0), Utc),
    };
    let uri = match repo.repository_uri {
        None => return Err(anyhow!(format!("ECR repository has no url: {:?}", repo))),
        Some(uri) => uri,
    };
    let (domain_name, repo_name) = uri
        .splitn(2, '/')
        .collect_tuple()
        .expect("Failed unpacking ECR repository URI");
    let new_registry = NewHelmRegistry {
        created_at,
        domain_name: domain_name.into(),
        repo_name: repo_name.into(),
    };
    info!("Saving {:?}", new_registry);
    Ok(new_registry.insert().await?)
}
