use anyhow::Result;
use platz_db::schema::deployment_kind::{DeploymentKind, NewDeploymentKind};
use titlecase::titlecase;

pub async fn get_or_create_kind(repo_name: &str) -> Result<DeploymentKind> {
    let kind_name = get_kind(repo_name);
    Ok(
        match DeploymentKind::find_by_name(kind_name.clone()).await {
            Ok(deploy_kind) => deploy_kind,
            Err(_) => {
                NewDeploymentKind {
                    name: kind_name.clone(),
                }
                .insert()
                .await?
            }
        },
    )
}

pub fn get_kind(repo_name: &str) -> String {
    let mut parts = repo_name.rsplit('-').peekable();

    // Remove '-chart' or '-charts' suffixes, like moo-charts
    match parts.peek() {
        Some(&"chart") | Some(&"charts") => {
            parts.next();
        }
        _ => (),
    }

    parts.map(titlecase).fold(String::new(), |a, b| b + &a)
}

#[test]
fn test_get_kind() {
    assert_eq!(get_kind("word"), "Word");
    assert_eq!(get_kind("word-chart"), "Word");
    assert_eq!(get_kind("word-charts"), "Word");

    assert_eq!(get_kind("two-words"), "TwoWords");
    assert_eq!(get_kind("two-words-chart"), "TwoWords");
    assert_eq!(get_kind("two-words-charts"), "TwoWords");

    assert_eq!(get_kind("chart-not-last"), "ChartNotLast");
    assert_eq!(get_kind("three-word-kind"), "ThreeWordKind");
}
