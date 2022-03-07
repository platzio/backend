mod fake_db;

use anyhow::Result;
use fake_db::TestDb;
use platz_ui_schema::UiSchema;
use serde_json::json;

#[tokio::test]
async fn test_single_collection() -> Result<()> {
    let schema = json!({
        "inputs": [
            {
                "id": "a",
                "type": "CollectionSelect",
                "label": "Select an A",
                "collection": "First",
                "required": true,
            },
        ],
        "outputs": {
            "values": [
                {
                    "path": [
                        "config",
                        "a",
                        "id",
                    ],
                    "value": {
                        "FieldProperty": {
                            "input": "a",
                            "property": "id",
                        }
                    }
                },
                {
                    "path": [
                        "config",
                        "a",
                        "value",
                    ],
                    "value": {
                        "FieldProperty": {
                            "input": "a",
                            "property": "a",
                        }
                    }
                },
            ],
            "secrets": {}
        }
    });
    let ui_schema: UiSchema = serde_json::from_value(schema)?;
    let inputs = json!({
        "a": "3",
    });
    let values: serde_json::Value = ui_schema.get_values::<TestDb>(&inputs).await?.into();
    let expected = json!({
        "config": {
            "a": {
                "id": "3",
                "value": "a3",
            }
        }
    });
    assert_eq!(values, expected);
    Ok(())
}

#[tokio::test]
async fn test_array_of_collection() -> Result<()> {
    let schema = json!({
        "inputs": [
            {
                "id": "a",
                "type": "array",
                "label": "Select some A",
                "itemType": "CollectionSelect",
                "collection": "First",
                "required": true,
            },
        ],
        "outputs": {
            "values": [
                {
                    "path": [
                        "config",
                        "a",
                        "id",
                    ],
                    "value": {
                        "FieldProperty": {
                            "input": "a",
                            "property": "id",
                        }
                    }
                },
                {
                    "path": [
                        "config",
                        "a",
                        "value",
                    ],
                    "value": {
                        "FieldProperty": {
                            "input": "a",
                            "property": "a",
                        }
                    }
                },
            ],
            "secrets": {}
        }
    });
    let ui_schema: UiSchema = serde_json::from_value(schema)?;
    let inputs = json!({
        "a": ["3", "4"],
    });
    let values: serde_json::Value = ui_schema.get_values::<TestDb>(&inputs).await?.into();
    let expected = json!({
        "config": {
            "a": {
                "id": ["3", "4"],
                "value": ["a3", "a4"],
            }
        }
    });
    assert_eq!(values, expected);
    Ok(())
}

#[tokio::test]
async fn test_all_input_types() -> Result<()> {
    let schema = json!({
        "inputs": [
            {
                "id": "secret1",
                "type": "CollectionSelect",
                "label": "Secret 1",
                "required": true,
                "collection": "secrets"
            },
            {
                "id": "radio1",
                "type": "RadioSelect",
                "label": "Radio 1",
                "options": [
                    {
                        "label": "Option 1",
                        "value": "option1"
                    },
                    {
                        "label": "Option 2",
                        "value": "option2"
                    }
                ],
                "required": true
            },
            {
                "id": "when_option1",
                "type": "CollectionSelect",
                "label": "When Option 1 Selected",
                "filters": [
                    {
                        "field": "kind",
                        "value": "ForOption1"
                    }
                ],
                "showIfAll": [
                    {
                        "field": "radio1",
                        "value": "option1"
                    }
                ],
                "collection": "deployments"
            },
            {
                "id": "select1",
                "type": "CollectionSelect",
                "label": "Select 1",
                "filters": [
                    {
                        "field": "kind",
                        "value": "ForOption2"
                    }
                ],
                "collection": "deployments"
            },
            {
                "id": "select2",
                "type": "CollectionSelect",
                "label": "Select 2",
                "filters": [
                    {
                        "field": "kind",
                        "value": "ForSelect2"
                    }
                ],
                "collection": "deployments"
            },
            {
                "id": "text1",
                "type": "text",
                "label": "Text 1",
                "showIfAll": [
                    {
                        "field": "radio1",
                        "value": "option2"
                    }
                ]
            },
            {
                "id": "text2",
                "type": "text",
                "label": "Text 2",
                "showIfAll": [
                    {
                        "field": "radio1",
                        "value": "option2"
                    }
                ]
            },
            {
                "id": "text3",
                "type": "text",
                "label": "Text 3",
                "helpText": "Example: Text 3",
                "required": true
            },
            {
                "id": "alias",
                "type": "text",
                "label": "Alias",
                "helpText": "Short name for this deployment."
            },
            {
                "id": "number1",
                "type": "number",
                "label": "Number 1",
                "required": true,
                "initialValue": "0"
            },
            {
                "id": "number2",
                "type": "number",
                "label": "Number 2",
                "required": true
            },
            {
                "id": "number3",
                "type": "number",
                "label": "Number 3",
                "minimum": 0
            },
            {
                "id": "number4",
                "type": "number",
                "label": "Number 4"
            },
            {
                "id": "number5",
                "type": "number",
                "label": "Number 5"
            },
            {
                "id": "checkbox1",
                "type": "Checkbox",
                "label": "checkbox1",
                "initialValue": false
            },
            {
                "id": "number6",
                "type": "number",
                "label": "Number 6",
                "minimum": 0,
                "showIfAll": [
                    {
                        "field": "checkbox1",
                        "value": true
                    }
                ]
            },
            {
                "id": "number7",
                "type": "number",
                "label": "Number 7",
                "minimum": 0,
                "showIfAll": [
                    {
                        "field": "checkbox1",
                        "value": true
                    }
                ]
            },
            {
                "id": "number8",
                "type": "number",
                "label": "Number 8",
                "maximum": 1,
                "minimum": 0,
                "showIfAll": [
                    {
                        "field": "checkbox1",
                        "value": true
                    }
                ]
            },
            {
                "id": "number9",
                "type": "number",
                "label": "Number 9"
            },
            {
                "id": "radio2",
                "type": "RadioSelect",
                "label": "Radio 2",
                "options": [
                    {
                        "label": "value0",
                        "value": "Value 0"
                    },
                    {
                        "label": "value1",
                        "value": "Value 1"
                    },
                    {
                        "label": "value2",
                        "value": "Value 2"
                    }
                ],
                "initialValue": "Value 0"
            },
            {
                "id": "radio2_value1",
                "type": "number",
                "label": "When Value 1 is Selected",
                "minimum": 0,
                "showIfAll": [
                    {
                        "field": "radio2",
                        "value": "value1"
                    }
                ]
            },
            {
                "id": "radio2_value2",
                "type": "number",
                "label": "When Value 2 is Selected",
                "maximum": 1,
                "minimum": 0,
                "helpText": "Value 2 sas Selected",
                "showIfAll": [
                    {
                        "field": "radio2",
                        "value": "value2"
                    }
                ]
            },
            {
                "id": "text4",
                "type": "text",
                "label": "Text 4",
                "helpText": "Enter text 4",
                "required": true,
                "initialValue": "four"
            },
            {
                "id": "select2",
                "type": "CollectionSelect",
                "label": "Select 2",
                "filters": [
                    {
                        "field": "collection",
                        "value": "Some Collection"
                    }
                ],
                "collection": "secrets"
            },
            {
                "id": "schedule1",
                "type": "array",
                "label": "Schedule 1",
                "itemType": "DaysAndHour"
            },
            {
                "id": "schedule2",
                "type": "array",
                "label": "Schedule 2",
                "itemType": "DaysAndHour"
            }
        ],
        "outputs": {
            "values": [],
            "secrets": {
            }
        }
    });

    serde_json::from_value::<UiSchema>(schema)?;
    Ok(())
}
