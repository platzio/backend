mod fake_db;
mod utils;

use anyhow::Result;
use fake_db::TestDb;
use platz_chart_ext::{
    v1beta2::{
        ChartExtDeploymentDisplay, ChartExtDeploymentDisplayIcon, ChartExtDeploymentDisplayName,
        ChartExtDeploymentDisplayNameInputField,
    },
    ChartExtIngressHostnameFormat, UiSchema,
};
use serde_json::json;
use utils::load_chart;
use uuid::Uuid;

#[tokio::test]
async fn test1() -> Result<()> {
    let chart_ext = load_chart("v1beta2/chart1").await?;
    let values_ui = chart_ext.values_ui.expect("No values_ui");
    assert!(matches!(values_ui, UiSchema::V1Beta1(_)));
    let inputs = json!({
        "required_bool": true,
        "required_num": 3,
        "required_text": "blah",
        "ignored_field": 5,
        "array_of_text": ["value"]
    });
    let values: serde_json::Value = values_ui
        .get_values::<TestDb>(Uuid::new_v4(), &inputs)
        .await?
        .into();
    let expected = json!({
        "config": {
            "required_bool": true,
            "required_num": 3,
            "required_text": "blah",
            "array_of_text": ["value"]
        }
    });
    assert_eq!(values, expected);

    chart_ext.actions.expect("No actions");

    let features = chart_ext.features.expect("No features");
    assert!(features.ingress().enabled);
    assert!(matches!(
        features.ingress().hostname_format,
        ChartExtIngressHostnameFormat::KindAndName
    ));
    assert_eq!(
        features.display(),
        ChartExtDeploymentDisplay {
            name: None,
            icon: Some(ChartExtDeploymentDisplayIcon {
                font_awesome: "rocket".to_owned(),
            })
        }
    );

    let resource_types = chart_ext.resource_types.expect("No resource types");
    assert_eq!(resource_types.inner.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test2() -> Result<()> {
    let chart_ext = load_chart("v1beta2/chart3").await?;
    let values_ui = chart_ext.values_ui.expect("No values_ui");
    match values_ui {
        UiSchema::V0(_) => panic!("Expected UiSchema::V1Beta1"),
        UiSchema::V1Beta1(schema) => {
            assert_eq!(schema.inner.outputs.secrets.len(), 2);
        }
    }

    chart_ext.actions.expect("No actions");

    let features = chart_ext.features.expect("No features");
    assert!(!features.ingress().enabled);
    assert!(matches!(
        features.ingress().hostname_format,
        ChartExtIngressHostnameFormat::Name
    ));
    assert_eq!(
        features.display(),
        ChartExtDeploymentDisplay {
            name: Some(ChartExtDeploymentDisplayName::InputField(
                ChartExtDeploymentDisplayNameInputField {
                    name: "alias".to_owned(),
                }
            )),
            icon: Some(ChartExtDeploymentDisplayIcon {
                font_awesome: "rocket".to_owned(),
            })
        }
    );

    let resource_types = chart_ext.resource_types.expect("No resource types");
    assert_eq!(resource_types.inner.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test3() -> Result<()> {
    let chart_ext = load_chart("v1beta2/chart4").await?;

    let features = chart_ext.features.expect("No features");
    assert_eq!(
        features.display(),
        ChartExtDeploymentDisplay {
            name: Some(ChartExtDeploymentDisplayName::DeploymentName),
            icon: Some(ChartExtDeploymentDisplayIcon {
                font_awesome: "rocket".to_owned(),
            })
        }
    );

    Ok(())
}
