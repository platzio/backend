mod fake_db;
mod utils;

use anyhow::Result;
use fake_db::TestDb;
use platz_chart_ext::{ChartExt, UiSchema};
use serde_json::json;
use utils::chart_dir;

#[tokio::test]
async fn test() -> Result<()> {
    let chart_ext = ChartExt::from_path(&chart_dir("v1/chart1")).await?;
    let values_ui = chart_ext.values_ui.expect("No values_ui");
    assert!(matches!(values_ui, UiSchema::V1(_)));
    let inputs = json!({
        "required_bool": true,
        "required_num": 3,
        "required_text": "blah",
        "ignored_field": 5,
        "array_of_text": ["value"]
    });
    let values: serde_json::Value = values_ui.get_values::<TestDb>(&inputs).await?.into();
    let expected = json!({
        "config": {
            "required_bool": true,
            "required_num": 3,
            "required_text": "blah",
            "array_of_text": ["value"]
        }
    });
    assert_eq!(values, expected);
    Ok(())
}
