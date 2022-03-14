#[derive(Debug, thiserror::Error)]
pub enum UiSchemaInputError<CollectionError>
where
    CollectionError: std::fmt::Display,
{
    #[error("An input is missing while being referenced in an output field: {0}")]
    MissingInputValue(String),

    #[error("An optional input was not provided: {0}")]
    OptionalInputMissing(String),

    #[error("An output refers to the {0} input, but it doesn't appear in the schema")]
    MissingInputSchema(String),

    #[error("An output refers to the input field {0}, but that input is not a collection")]
    InputNotACollection(String),

    #[error("Could not find a collection named {0}: {1}")]
    InvalidCollectionName(serde_json::Value, serde_json::Error),

    #[error("Could not find a {0} collection item with ID: {1}")]
    CollectionItemNotFound(String, String),

    #[error("The {0} input field was expected to be a string")]
    InputNotString(String),

    #[error("The {0} input field was expected to be an array of strings")]
    InputNotStringArray(String),

    #[error("Input expected to be a UUID: {0}")]
    InvalidCollectionId(String),

    #[error("Collection not supported for references: {0}")]
    UnsupportedCollection(String),

    #[error("Unknown property {0} of collection {1}")]
    UnknownProperty(String, String),

    #[error("Error while resolving collection property: {0}")]
    CollectionError(#[from] CollectionError),
}
