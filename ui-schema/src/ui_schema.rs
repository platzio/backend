use crate::UiSchemaCollections;
use crate::UiSchemaInputError;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct UiSchema {
    pub inputs: Vec<UiSchemaInput>,
    pub outputs: UiSchemaOutputs,
}

#[derive(
    Debug, Deserialize, Serialize, strum_macros::EnumString, strum_macros::EnumDiscriminants,
)]
#[strum_discriminants(derive(strum_macros::EnumString, strum_macros::Display,))]
#[strum_discriminants(strum(ascii_case_insensitive))]
pub enum UiSchemaInputSingleType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "number")]
    Number,
    CollectionSelect {
        collection: String,
    },
    RadioSelect,
    DaysAndHour,
    Checkbox,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(try_from = "SerializedUiSchemaInputType")]
pub struct UiSchemaInputType {
    pub single_type: UiSchemaInputSingleType,
    pub is_array: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct SerializedUiSchemaInputType {
    r#type: String,
    #[serde(rename = "itemType")]
    item_type: Option<String>,
    collection: Option<String>,
}

impl TryFrom<SerializedUiSchemaInputType> for UiSchemaInputType {
    type Error = strum::ParseError;

    fn try_from(s: SerializedUiSchemaInputType) -> Result<Self, Self::Error> {
        let is_array = s.r#type == "array";
        let single_type_disc = if is_array {
            s.item_type.ok_or(strum::ParseError::VariantNotFound)?
        } else {
            s.r#type
        };

        let disc: UiSchemaInputSingleTypeDiscriminants = single_type_disc.parse()?;
        let single_type = match disc {
            UiSchemaInputSingleTypeDiscriminants::CollectionSelect => {
                UiSchemaInputSingleType::CollectionSelect {
                    collection: s.collection.ok_or(strum::ParseError::VariantNotFound)?,
                }
            }
            UiSchemaInputSingleTypeDiscriminants::Text => UiSchemaInputSingleType::Text,
            UiSchemaInputSingleTypeDiscriminants::Number => UiSchemaInputSingleType::Number,
            UiSchemaInputSingleTypeDiscriminants::RadioSelect => {
                UiSchemaInputSingleType::RadioSelect
            }
            UiSchemaInputSingleTypeDiscriminants::Checkbox => UiSchemaInputSingleType::Checkbox,
            UiSchemaInputSingleTypeDiscriminants::DaysAndHour => {
                UiSchemaInputSingleType::DaysAndHour
            }
        };
        Ok(Self {
            single_type,
            is_array,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FieldValuePair {
    field: String,
    value: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UiSchemaInput {
    pub id: String,
    #[serde(flatten)]
    pub input_type: UiSchemaInputType, // Parsed from actual fields: type, item_type and collection, see SerializedUiSchemaInputType
    #[serde(default)]
    pub required: bool,

    // All these stuff are not for the backend, but can be serialized
    label: String,
    #[serde(default, rename = "helpText")]
    help_text: Option<String>,
    #[serde(default, rename = "initialValue")]
    initial_value: Option<serde_json::Value>,
    #[serde(default, rename = "showIfAll")]
    show_if_all: Option<Vec<FieldValuePair>>,
}

pub type UiSchemaOutputSecrets = HashMap<String, HashMap<String, UiSchemaInputRef>>;

#[derive(Debug, Deserialize)]
pub struct UiSchemaOutputs {
    pub values: Vec<UiSchemaOutputValue>,
    #[serde(default)]
    pub secrets: UiSchemaOutputSecrets,
}

#[derive(Debug, Deserialize)]
pub struct InputFieldValue {
    pub input: String,
}

#[derive(Debug, Deserialize)]
pub struct InputFieldProperty {
    pub input: String,
    pub property: String,
}

#[derive(Debug, Deserialize)]
pub enum UiSchemaInputRef {
    FieldValue(InputFieldValue),
    FieldProperty(InputFieldProperty),
}

impl UiSchemaInputRef {
    fn get_input_schema<'a, C>(
        input_schema: &'a [UiSchemaInput],
        id: &str,
    ) -> Result<&'a UiSchemaInput, UiSchemaInputError<C>>
    where
        C: UiSchemaCollections,
    {
        input_schema
            .iter()
            .find(|i| i.id == id)
            .ok_or_else(|| UiSchemaInputError::MissingInputSchema(id.to_owned()))
    }

    fn get_input<C>(
        schema: &UiSchemaInput,
        inputs: &serde_json::Value,
        id: &str,
    ) -> Result<serde_json::Value, UiSchemaInputError<C>>
    where
        C: UiSchemaCollections,
    {
        if let Some(show_if_all) = schema.show_if_all.as_ref() {
            if show_if_all
                .iter()
                .any(|fv| inputs.get(&fv.field) != Some(&fv.value))
            {
                return Err(UiSchemaInputError::OptionalInputMissing(id.to_owned()));
            }
        }
        Ok(inputs
            .get(id)
            .ok_or_else(|| {
                if schema.required {
                    UiSchemaInputError::MissingInputValue(id.to_owned())
                } else {
                    UiSchemaInputError::OptionalInputMissing(id.to_owned())
                }
            })?
            .clone())
    }

    pub async fn resolve<C>(
        &self,
        input_schema: &[UiSchemaInput],
        inputs: &serde_json::Value,
    ) -> Result<serde_json::Value, UiSchemaInputError<C>>
    where
        C: UiSchemaCollections,
    {
        match self {
            Self::FieldValue(fv) => Self::get_input(
                Self::get_input_schema(input_schema, &fv.input)?,
                inputs,
                &fv.input,
            ),
            Self::FieldProperty(fp) => {
                let schema = Self::get_input_schema(input_schema, &fp.input)?;
                match &schema.input_type.single_type {
                    UiSchemaInputSingleType::CollectionSelect { collection } => {
                        let collection_value = serde_json::to_value(collection).unwrap();
                        let collections: C =
                            serde_json::from_value(collection_value).map_err(|err| {
                                UiSchemaInputError::InvalidCollectionName(
                                    collection.to_owned(),
                                    err,
                                )
                            })?;
                        let id_value = Self::get_input(schema, inputs, &fp.input)?;
                        if schema.input_type.is_array {
                            let id_value_arr = id_value.as_array().ok_or_else(|| {
                                UiSchemaInputError::InputNotStringArray(fp.input.clone())
                            })?;

                            let mut resolved_arr = Vec::new();

                            for id_value in id_value_arr {
                                let id = id_value.as_str().ok_or_else(|| {
                                    UiSchemaInputError::InputNotStringArray(fp.input.clone())
                                })?;
                                let resolved_value = collections.resolve(id, &fp.property).await?;
                                resolved_arr.push(resolved_value);
                            }
                            Ok(serde_json::to_value(resolved_arr).unwrap())
                        } else {
                            let id = id_value.as_str().ok_or_else(|| {
                                UiSchemaInputError::InputNotString(fp.input.clone())
                            })?;
                            collections.resolve(id, &fp.property).await
                        }
                    }
                    _ => Err(UiSchemaInputError::InputNotACollection(fp.input.clone())),
                }
            }
        }
    }
}

type Map = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Deserialize)]
pub struct UiSchemaOutputValue {
    pub path: Vec<String>,
    pub value: UiSchemaInputRef,
}

pub fn insert_into_map(map: &mut Map, path: &[String], value: serde_json::Value) {
    let mut cur_node = map;
    let mut iter = path.iter().peekable();

    while let Some(part) = iter.next() {
        if iter.peek().is_none() {
            cur_node.insert(part.to_owned(), value);
            return;
        }
        if !cur_node.contains_key(part) {
            cur_node.insert(part.to_owned(), serde_json::Value::Object(Map::new()));
        }
        cur_node = cur_node.get_mut(part).unwrap().as_object_mut().unwrap();
    }
}

impl UiSchemaOutputValue {
    pub async fn resolve_into<C>(
        &self,
        input_schema: &[UiSchemaInput],
        inputs: &serde_json::Value,
        outputs: &mut Map,
    ) -> Result<(), UiSchemaInputError<C>>
    where
        C: UiSchemaCollections,
    {
        match self.value.resolve(input_schema, inputs).await {
            Ok(value) => {
                insert_into_map(outputs, &self.path, value);
                Ok(())
            }
            Err(UiSchemaInputError::OptionalInputMissing(_)) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl UiSchema {
    pub async fn get_values<C>(
        &self,
        inputs: &serde_json::Value,
    ) -> Result<Map, UiSchemaInputError<C>>
    where
        C: UiSchemaCollections,
    {
        let mut values = Map::new();
        for output in self.outputs.values.iter() {
            output
                .resolve_into(&self.inputs, inputs, &mut values)
                .await?;
        }
        Ok(values)
    }
}
