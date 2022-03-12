mod fake_db;

use anyhow::Result;
use fake_db::TestDb;
use platz_chart_ext::UiSchema;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test() -> Result<()> {
    let schema = json!({
        "inputs": [
            {
                "id": "required_num",
                "type": "number",
                "label": "Required num",
                "minimum": 0,
                "required": true,
                "initialValue": 600
            },
            {
                "id": "required_bool",
                "type": "Checkbox",
                "label": "Required bool",
                "initialValue": true
            },
            {
                "id": "optional_bool",
                "type": "Checkbox",
                "label": "Optional bool",
                "initialValue": false
            },
            {
                "id": "required_text",
                "type": "text",
                "label": "Required text",
                "required": true,
                "initialValue": "blah"
            },
            {
                "id": "array_of_text",
                "type": "array",
                "itemType": "text",
                "label": "Array of text",
            },
            {
                "id": "optional_text",
                "type": "text",
                "label": "Required text"
            },
        ],
        "outputs": {
            "values": [
                {
                    "path": [
                        "config",
                        "required_num"
                    ],
                    "value": {
                        "FieldValue": {
                            "input": "required_num"
                        }
                    }
                },
                {
                    "path": [
                        "config",
                        "required_bool"
                    ],
                    "value": {
                        "FieldValue": {
                            "input": "required_bool"
                        }
                    }
                },
                {
                    "path": [
                        "config",
                        "required_text"
                    ],
                    "value": {
                        "FieldValue": {
                            "input": "required_text"
                        }
                    }
                },
                {
                    "path": [
                        "config",
                        "array_of_text"
                    ],
                    "value": {
                        "FieldValue": {
                            "input": "array_of_text"
                        }
                    }
                },

                {
                    "path": [
                        "config",
                        "optional_bool"
                    ],
                    "value": {
                        "FieldValue": {
                            "input": "optional_bool"
                        }
                    }
                },
            ],
            "secrets": {}
        }
    });
    let ui_schema: UiSchema = serde_json::from_value(schema)?;
    let inputs = json!({
        "required_bool": true,
        "required_num": 3,
        "required_text": "blah",
        "ignored_field": 5,
        "array_of_text": ["value"]
    });
    let values: serde_json::Value = ui_schema
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
    Ok(())
}
