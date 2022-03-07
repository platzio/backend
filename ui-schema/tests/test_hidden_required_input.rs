mod fake_db;

pub use anyhow::Result;
pub use fake_db::TestDb;
pub use platz_ui_schema::*;
pub use serde_json::json;

#[tokio::test]
async fn test() -> Result<()> {
    let schema = json!({
        "inputs": [
            {
                "id": "required_enum",
                "type": "RadioSelect",
                "label": "Required enum",
                "options": [
                    {
                        "value": "value1",
                        "label": "Value 1"
                    },
                    {
                        "value": "value2",
                        "label": "Value 2"
                    },
                    {
                        "value": "value3",
                        "label": "Value 3",
                        "helpText": "Third value"
                    }
                ],
                "required": true
            },
            {
                "id": "required_dependent_num",
                "label": "Required dependent num",
                "type": "number",
                "minimum": 0,
                "showIfAll": [
                    {
                        "field": "required_enum",
                        "value": "value3"
                    }
                ],
                "required": true
            },
        ],
        "outputs": {
            "values": [
                {
                    "path": [
                        "config",
                        "required_enum"
                    ],
                    "value": {
                        "FieldValue": {
                            "input": "required_enum"
                        }
                    }
                },
                {
                    "path": [
                        "config",
                        "required_dependent_num"
                    ],
                    "value": {
                        "FieldValue": {
                            "input": "required_dependent_num"
                        }
                    }
                },
            ],
            "secrets": {}
        }
    });
    let ui_schema: UiSchema = serde_json::from_value(schema)?;

    let inputs1 = json!({
        "required_enum": "value2",
    });
    let values1: serde_json::Value = ui_schema.get_values::<TestDb>(&inputs1).await?.into();
    let expected1 = json!({
        "config": {
            "required_enum": "value2",
        }
    });
    assert_eq!(values1, expected1);

    let inputs2 = json!({
        "required_enum": "value3",
    });
    let _missing = "required_dependent_num".to_owned();
    assert!(std::matches!(
        ui_schema.get_values::<TestDb>(&inputs2).await,
        Err(UiSchemaInputError::MissingInputValue(_missing))
    ));

    let inputs3 = json!({
        "required_enum": "value3",
        "required_dependent_num": 5,
    });
    let values3: serde_json::Value = ui_schema.get_values::<TestDb>(&inputs3).await?.into();
    let expected3 = json!({
        "config": {
            "required_enum": "value3",
            "required_dependent_num": 5,
        }
    });
    assert_eq!(values3, expected3);

    Ok(())
}
