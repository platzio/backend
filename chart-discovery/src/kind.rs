use anyhow::Result;
use platz_db::{HelmRegistry, UpdateHelmRegistryKind};
use titlecase::titlecase;

pub async fn update_all_registries() -> Result<()> {
    for helm_registry in HelmRegistry::all().await?.into_iter() {
        let kind = get_kind(&helm_registry.repo_name);
        UpdateHelmRegistryKind { kind }
            .save(helm_registry.id)
            .await?;
    }
    Ok(())
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
