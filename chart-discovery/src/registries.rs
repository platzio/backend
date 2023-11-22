use crate::kind::get_kind;
use anyhow::{anyhow, Result};
use aws_sdk_ecr::types::Repository;
use aws_smithy_types_convert::date_time::DateTimeExt;
use aws_types::region::Region;
use itertools::Itertools;
use log::*;
use platz_db::{HelmRegistry, NewHelmRegistry};

pub async fn find_and_save_ecr_repo(region: Region, repo_name: &str) -> Result<HelmRegistry> {
    let shared_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let config = aws_sdk_ecr::config::Builder::from(&shared_config)
        .region(region)
        .build();
    let ecr = aws_sdk_ecr::Client::from_conf(config);

    let result = ecr
        .describe_repositories()
        .repository_names(repo_name)
        .send()
        .await?;

    let repo = result
        .repositories
        .ok_or_else(|| anyhow!("Could not find ECR repo: {}", repo_name))?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("ECR repo not found: {}", repo_name))?;

    save_ecr_repo_in_db(repo).await
}

async fn save_ecr_repo_in_db(repo: Repository) -> Result<HelmRegistry> {
    let created_at = repo
        .created_at
        .ok_or_else(|| anyhow!("ECR repository has no created_at: {:?}", repo))?
        .to_chrono_utc()?;

    let uri = repo
        .repository_uri
        .as_ref()
        .ok_or_else(|| anyhow!("ECR repository has no url: {:?}", repo))?;

    let (domain_name, repo_name) = uri
        .splitn(2, '/')
        .collect_tuple()
        .expect("Failed unpacking ECR repository URI");

    let new_registry = NewHelmRegistry {
        created_at,
        domain_name: domain_name.to_owned(),
        repo_name: repo_name.to_owned(),
        kind: get_kind(repo_name),
    };
    info!("Saving {:?}", new_registry);
    Ok(new_registry.insert().await?)
}
