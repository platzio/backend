use super::actions::HelmChartActionsSchema;
use super::features::HelmChartFeatures;
use super::values_ui::UiSchema;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use tokio::fs::read_to_string;
use tokio::try_join;

#[derive(Debug)]
pub struct ChartExt {
    pub values_ui: Option<UiSchema>,
    pub actions: Option<HelmChartActionsSchema>,
    pub features: Option<HelmChartFeatures>,
    pub error: Option<String>,
}

impl ChartExt {
    pub async fn from_path(path: &Path) -> Result<Self, std::io::Error> {
        match read_chart_extensions(path).await {
            Ok((values_ui, actions, features)) => Ok(Self {
                values_ui,
                actions,
                features,
                error: None,
            }),
            Err(error @ ChartExtError::ParseError(_, _)) => Ok(Self {
                values_ui: None,
                actions: None,
                features: None,
                error: Some(error.to_string()),
            }),
            Err(ChartExtError::IoError(err)) => Err(err),
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn to_values(
        self,
    ) -> Result<
        (
            Option<serde_json::Value>,
            Option<serde_json::Value>,
            Option<serde_json::Value>,
        ),
        String,
    > {
        match self.error {
            None => Ok((
                self.values_ui
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|err| format!("Error converting values_ui to JSON: {}", err))?,
                self.actions
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|err| format!("Error converting actions to JSON: {}", err))?,
                self.features
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|err| format!("Error converting features to JSON: {}", err))?,
            )),
            Some(error) => Err(error),
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
        Option<HelmChartActionsSchema>,
        Option<HelmChartFeatures>,
    ),
    ChartExtError,
> {
    match try_read_chart_extensions(
        path,
        "platz/values-ui.yaml",
        "platz/actions.yaml",
        "platz/features.yaml",
    )
    .await?
    {
        (None, None, None) => {
            // Try reading legacy json files
            try_read_chart_extensions(
                path,
                "values.ui.json",
                "actions.schema.json",
                "features.json",
            )
            .await
        }
        res => Ok(res),
    }
}

async fn try_read_chart_extensions(
    chart_path: &Path,
    values_ui_filename: &str,
    actions_filename: &str,
    features_filename: &str,
) -> Result<
    (
        Option<UiSchema>,
        Option<HelmChartActionsSchema>,
        Option<HelmChartFeatures>,
    ),
    ChartExtError,
> {
    Ok(try_join!(
        read_spec_file(chart_path, values_ui_filename),
        read_spec_file(chart_path, actions_filename),
        read_spec_file(chart_path, features_filename),
    )?)
}

async fn read_spec_file<T>(path: &Path, filename: &str) -> Result<Option<T>, ChartExtError>
where
    T: Serialize + DeserializeOwned,
{
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
