use anyhow::Result;
use platz_chart_ext::resource_types::ChartExtResourceTypes;
use platz_chart_ext::{ChartExt, ChartExtActions, ChartExtFeatures, UiSchema};
use std::path::PathBuf;

fn chart_dir(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("charts")
        .join(relative_path)
}

pub async fn load_chart(relative_path: &str) -> Result<ChartExt> {
    let chart_ext = ChartExt::from_path(&chart_dir(relative_path)).await?;
    println!("{:#?}", chart_ext);

    if chart_ext.error.is_none() {
        if let Some(values_ui) = chart_ext.values_ui.as_ref() {
            let json = serde_json::to_value(values_ui)
                .expect("Error serializing UiSchema to JSON after successfully deserializing it");
            let _parsed: UiSchema =
                serde_json::from_value(json).expect("Failed deserializing UiSchema from JSON");
        }

        if let Some(actions) = chart_ext.actions.as_ref() {
            let json = serde_json::to_value(actions).expect(
                "Error serializing ChartExtActions to JSON after successfully deserializing it",
            );
            let _parsed: ChartExtActions = serde_json::from_value(json)
                .expect("Failed deserializing ChartExtActions from JSON");
        }

        if let Some(features) = chart_ext.features.as_ref() {
            let json = serde_json::to_value(features).expect(
                "Error serializing ChartExtFeatures to JSON after successfully deserializing it",
            );
            let _parsed: ChartExtFeatures = serde_json::from_value(json)
                .expect("Failed deserializing ChartExtFeatures from JSON");
        }

        if let Some(resource_types) = chart_ext.resource_types.as_ref() {
            let json = serde_json::to_value(resource_types).expect(
                "Error serializing ChartExtResourceTypes to JSON after successfully deserializing it",
            );
            let _parsed: ChartExtResourceTypes = serde_json::from_value(json)
                .expect("Failed deserializing ChartExtResourceTypes from JSON");
        }
    }

    Ok(chart_ext)
}
