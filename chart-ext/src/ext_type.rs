use super::actions::ChartExtActions;
use super::features::ChartExtFeatures;
use super::values_ui::UiSchema;
use crate::resource_types::ChartExtResourceTypes;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use tokio::fs::read_to_string;
use tokio::try_join;

#[derive(Debug)]
pub struct ChartExt {
    pub values_ui: Option<UiSchema>,
    pub actions: Option<ChartExtActions>,
    pub features: Option<ChartExtFeatures>,
    pub resource_types: Option<ChartExtResourceTypes>,
    pub error: Option<String>,
}

impl ChartExt {
    pub async fn from_path(path: &Path) -> Result<Self, std::io::Error> {
        match read_chart_extensions(path).await {
            Ok((values_ui, actions, features, resource_types)) => Ok(Self {
                values_ui,
                actions,
                features,
                resource_types,
                error: None,
            }),
            Err(error @ ChartExtError::ParseError(_, _)) => Ok(Self {
                values_ui: None,
                actions: None,
                features: None,
                resource_types: None,
                error: Some(error.to_string()),
            }),
            Err(ChartExtError::IoError(err)) => Err(err),
        }
    }

    pub fn new_with_error(error: String) -> Self {
        Self {
            values_ui: None,
            actions: None,
            features: None,
            resource_types: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ChartExtError {
    #[error("std::io::Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Error while parsing {0}: {1}")]
    ParseError(String, String),
}

// let value = serde_json::to_value(spec)
// .map_err(|err| anyhow!("Error while converting to JSON ({}): {:?}", filename, err))?;

async fn read_chart_extensions(
    path: &Path,
) -> Result<
    (
        Option<UiSchema>,
        Option<ChartExtActions>,
        Option<ChartExtFeatures>,
        Option<ChartExtResourceTypes>,
    ),
    ChartExtError,
> {
    match try_read_chart_extensions(
        path,
        Some("platz/values-ui.yaml"),
        Some("platz/actions.yaml"),
        Some("platz/features.yaml"),
        Some("platz/resources.yaml"),
    )
    .await?
    {
        (None, None, None, None) => {
            // Try reading legacy json files
            try_read_chart_extensions(
                path,
                Some("values.ui.json"),
                Some("actions.schema.json"),
                Some("features.json"),
                None,
            )
            .await
        }
        res => Ok(res),
    }
}

async fn try_read_chart_extensions(
    chart_path: &Path,
    values_ui_filename: Option<&str>,
    actions_filename: Option<&str>,
    features_filename: Option<&str>,
    resource_types_filename: Option<&str>,
) -> Result<
    (
        Option<UiSchema>,
        Option<ChartExtActions>,
        Option<ChartExtFeatures>,
        Option<ChartExtResourceTypes>,
    ),
    ChartExtError,
> {
    Ok(try_join!(
        read_spec_file(chart_path, values_ui_filename),
        read_spec_file(chart_path, actions_filename),
        read_spec_file(chart_path, features_filename),
        read_spec_file(chart_path, resource_types_filename),
    )?)
}

async fn read_spec_file<T>(path: &Path, filename: Option<&str>) -> Result<Option<T>, ChartExtError>
where
    T: Serialize + DeserializeOwned,
{
    let filename = match filename {
        Some(filename) => filename,
        None => return Ok(None),
    };

    let full_path = path.join(filename);

    let file_ext = full_path
        .extension()
        .and_then(|osstr| osstr.to_str())
        .map(ToString::to_string);

    let contents = match read_to_string(full_path).await {
        Ok(contents) => contents,
        Err(err) => {
            return if err.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(err.into())
            };
        }
    };

    match file_ext.as_deref() {
        Some("yaml") | Some("yml") => {
            Ok(Some(serde_yaml::from_str(&contents).map_err(|err| {
                ChartExtError::ParseError(filename.to_owned(), err.to_string())
            })?))
        }
        Some("json") => Ok(Some(serde_json::from_str(&contents).map_err(|err| {
            ChartExtError::ParseError(filename.to_owned(), err.to_string())
        })?)),
        _ => Err(ChartExtError::ParseError(
            filename.to_owned(),
            "Unknown file extension".to_owned(),
        )),
    }
}
