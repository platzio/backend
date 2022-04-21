use anyhow::Result;
use itertools::Itertools;
use k8s_openapi::api::core::v1::Namespace;
use maplit::btreemap;
use platz_db::Deployment;
use std::collections::BTreeMap;
use uuid::Uuid;

pub const NAMESPACE_LABEL_KEY: &str = "platz";
pub const NAMESPACE_ANNOTATION_DEPLOYMENT_ID: &str = "platz_deployment_id";

lazy_static::lazy_static! {
    pub static ref DEPLOYMENT_NAMESPACE_LABELS: BTreeMap<String, String> = btreemap! {
        NAMESPACE_LABEL_KEY.to_owned() => "yes".to_owned(),
    };

    pub static ref DEPLOYMENT_NAMESPACE_LABELS_SELECTOR: String =
        DEPLOYMENT_NAMESPACE_LABELS
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .join(",");
}

pub fn deployment_namespace_annotations(deployment: &Deployment) -> BTreeMap<String, String> {
    btreemap! {
        NAMESPACE_ANNOTATION_DEPLOYMENT_ID.to_owned() => deployment.id.to_string(),
    }
}

pub async fn find_deployment_from_namespace(namespace: &Namespace) -> Result<Option<Deployment>> {
    match namespace.metadata.annotations.as_ref() {
        None => Ok(None),
        Some(annotations) => match annotations.get(NAMESPACE_ANNOTATION_DEPLOYMENT_ID) {
            None => Ok(None),
            Some(raw_id) => Ok(Deployment::find_optional(Uuid::parse_str(raw_id)?).await?),
        },
    }
}
