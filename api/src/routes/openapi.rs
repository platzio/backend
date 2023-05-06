use platz_auth::USER_TOKEN_HEADER;
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
    OpenApi,
};

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
)]
struct OpenApiRoot;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "access_token",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
        components.add_security_scheme(
            "user_token",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new(USER_TOKEN_HEADER))),
        );
    }
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum SchemaFormat {
    /// Generate OpenAPI schema in YAML format
    Yaml,
    /// Generate OpenAPI schema in JSON format
    Json,
}

impl std::fmt::Display for SchemaFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Yaml => "yaml",
                Self::Json => "json",
            }
        )
    }
}

pub fn get_schema(format: SchemaFormat) -> String {
    let mut openapi = OpenApiRoot::openapi();
    openapi.merge(super::v2::ApiV2::openapi());
    match format {
        SchemaFormat::Yaml => openapi.to_yaml().unwrap(),
        SchemaFormat::Json => openapi.to_json().unwrap(),
    }
}
