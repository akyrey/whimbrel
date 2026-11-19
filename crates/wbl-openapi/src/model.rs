//! The normalized model: OpenAPI 3.2 as the superset every version is read into. Schemas are
//! JSON Schema values whose references point into [`Api::schemas`] (`#/schemas/<name>`).

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SpecVersion {
    Swagger2,
    OpenApi30,
    OpenApi31,
    OpenApi32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Api {
    pub version: SpecVersion,
    /// As written in the document, e.g. `3.1.0`.
    pub version_text: String,
    pub title: String,
    pub api_version: String,
    pub description: Option<String>,
    pub servers: Vec<Server>,
    pub tags: Vec<Tag>,
    pub operations: Vec<Operation>,
    /// Read-only: requests the API sends, not ones to send it.
    pub webhooks: Vec<Operation>,
    pub security_schemes: Vec<SecurityScheme>,
    /// The default for operations that do not set their own.
    pub security: Vec<SecurityRequirement>,
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    pub schemas: BTreeMap<String, Value>,
    /// What could not be understood; the import goes on without it.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ServerVariable {
    pub name: String,
    pub default: String,
    pub values: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Server {
    pub url: String,
    pub description: Option<String>,
    pub variables: Vec<ServerVariable>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Tag {
    pub name: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    /// 3.2: the tag this one nests under.
    pub parent: Option<String>,
    /// 3.2: e.g. `nav`, `badge`, `audience`.
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ParamLocation {
    Path,
    Query,
    /// 3.2: the whole query string from one schema.
    Querystring,
    Header,
    Cookie,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct NamedExample {
    pub name: String,
    pub summary: Option<String>,
    /// `value`, or 3.2 `dataValue`.
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub value: Option<Value>,
    /// 3.2 `serializedValue`: the example as sent.
    pub serialized: Option<String>,
    pub external: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    pub name: String,
    pub location: ParamLocation,
    pub required: bool,
    pub deprecated: bool,
    pub description: Option<String>,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub schema: Option<Value>,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub example: Option<Value>,
    pub examples: Vec<NamedExample>,
    pub style: Option<String>,
    pub explode: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MediaType {
    pub media_type: String,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub schema: Option<Value>,
    /// 3.2: the schema of each item of a stream (SSE, JSON Lines).
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub item_schema: Option<Value>,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub example: Option<Value>,
    pub examples: Vec<NamedExample>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RequestBody {
    pub required: bool,
    pub description: Option<String>,
    pub content: Vec<MediaType>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ResponseSpec {
    /// `200`, `4XX`, `default`.
    pub status: String,
    /// 3.2 `summary`, else the description's first line.
    pub summary: Option<String>,
    pub description: Option<String>,
    pub content: Vec<MediaType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SecurityUse {
    pub scheme: String,
    pub scopes: Vec<String>,
}

/// All of `uses` together satisfy it (an empty one: no auth needed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SecurityRequirement {
    pub uses: Vec<SecurityUse>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    /// `operationId`, else `<method>-<path>`; unique in the API.
    pub id: String,
    pub has_operation_id: bool,
    /// Upper case; any token (`QUERY`, custom methods from `additionalOperations`).
    pub method: String,
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub deprecated: bool,
    pub parameters: Vec<Parameter>,
    pub request_body: Option<RequestBody>,
    pub responses: Vec<ResponseSpec>,
    /// `None`: the API's default.
    pub security: Option<Vec<SecurityRequirement>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlow {
    /// `client_credentials`, `authorization_code`, `device_code`, `implicit`, `password`.
    pub kind: String,
    pub authorization_url: Option<String>,
    pub token_url: Option<String>,
    pub device_url: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SchemeKind {
    ApiKey {
        name: String,
        location: String,
    },
    Http {
        scheme: String,
        bearer_format: Option<String>,
    },
    #[serde(rename = "oauth2")]
    OAuth2 {
        flows: Vec<OAuthFlow>,
        metadata_url: Option<String>,
    },
    OpenIdConnect {
        url: String,
    },
    MutualTls,
    Unknown {
        kind: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SecurityScheme {
    pub name: String,
    pub description: Option<String>,
    pub deprecated: bool,
    pub kind: SchemeKind,
}
