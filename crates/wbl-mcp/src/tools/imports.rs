//! Imports into a collection and `.http` export (task 12.7), as the app makes them: OpenAPI
//! through `wbl-openapi`, Postman and `.http` through `wbl-interop`, cURL through `wbl-core`.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{ErrorData, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use wbl_engine::spec::SpecSource;
use wbl_format::{StoredRequest, collection};

use crate::error::McpError;
use crate::server::{Server, reply};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpenApiArgs {
    pub collection: String,
    /// A local file path, or an http(s) URL, of an OpenAPI 2.0, 3.0, 3.1 or 3.2 document.
    pub source: String,
    /// The new top-level folder the requests go in.
    pub folder_name: String,
    /// Also fetch documents the spec refers to on other servers.
    #[serde(default)]
    pub allow_remote: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileArgs {
    pub collection: String,
    /// A Postman v2.0/v2.1 collection or environment (.json), or a .http file.
    pub path: String,
    /// The new top-level folder the requests go in (unused for an environment alone).
    pub folder_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CurlArgs {
    pub collection: String,
    /// The folder to create the request in; "" (the default) for the top level.
    #[serde(default)]
    pub folder: String,
    /// The request's name; its method and URL when left out.
    pub name: Option<String>,
    /// A `curl …` command line.
    pub command: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportArgs {
    pub collection: String,
    /// The folder to export; "" (the default) for the whole collection.
    #[serde(default)]
    pub folder: String,
}

fn source_of(text: &str) -> SpecSource {
    let trimmed = text.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        SpecSource::Url {
            url: trimmed.to_owned(),
        }
    } else {
        SpecSource::File {
            path: trimmed.to_owned(),
        }
    }
}

fn stored_from_curl(args: &CurlArgs) -> Result<(StoredRequest, Vec<String>), McpError> {
    let parsed = wbl_core::curl::parse(&args.command).map_err(|e| {
        McpError::Engine(wbl_engine::EngineError::new("invalid_curl", e.to_string()))
    })?;
    let name = args
        .name
        .clone()
        .unwrap_or_else(|| format!("{} {}", parsed.request.method, parsed.request.url));
    let stored = StoredRequest {
        name,
        order: 0,
        request: parsed.request,
        examples: vec![],
        settings: parsed.settings,
        auth: Default::default(),
        variables: Default::default(),
        captures: vec![],
        assertions: vec![],
    };
    Ok((stored, parsed.ignored))
}

impl Server {
    async fn import_openapi_in(&self, args: OpenApiArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        let read = wbl_engine::spec::read(
            &self.state.sender,
            &source_of(&args.source),
            args.allow_remote,
        )
        .await?;
        let summary = wbl_engine::spec::summary(&read.api, read.remote.clone());
        let generated = wbl_openapi::generate(&read.api);
        let folder = collection::create_folder(&root, "", &args.folder_name)?;
        let installed = wbl_openapi::install(&root, &folder, &generated, &read.source)
            .map_err(|e| McpError::Engine(e.into()))?;
        Ok(json!({ "folder": folder, "summary": summary, "installed": installed }))
    }

    fn import_file_in(&self, args: FileArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        let imported = wbl_engine::interop::read(&args.path)?;
        let folder = if imported.requests.is_empty() {
            String::new()
        } else {
            collection::create_folder(&root, "", &args.folder_name)?
        };
        let installed = wbl_interop::install(&root, &folder, &imported)
            .map_err(|e| McpError::Engine(e.into()))?;
        Ok(json!({ "folder": folder, "installed": installed }))
    }

    fn import_curl_in(&self, args: CurlArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        let (stored, ignored) = stored_from_curl(&args)?;
        let text = wbl_format::RequestFile::new(&stored).to_string();
        wbl_engine::resolve::guard(&root, &text, &self.state.secrets)?;
        let path = collection::create_request(&root, &args.folder, &stored)?;
        let read = collection::read_request(&root, &path)?;
        Ok(json!({ "path": path, "request": read.stored, "ignored": ignored }))
    }
}

#[tool_router(router = import_tools, vis = "pub(crate)")]
impl Server {
    #[tool(
        description = "Imports an OpenAPI document (file or URL) into a new top-level folder: \
one request per operation, environments from its servers, and the spec kept in the collection. \
Returns what was made and any warnings."
    )]
    async fn import_openapi(
        &self,
        Parameters(args): Parameters<OpenApiArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.import_openapi_in(args).await))
    }

    #[tool(
        description = "Imports a Postman collection or environment, or a .http file, into a \
new top-level folder. Credentials become variables with secret names, never values; scripts are \
reported, not run."
    )]
    async fn import_file(
        &self,
        Parameters(args): Parameters<FileArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.import_file_in(args)))
    }

    #[tool(
        description = "Creates a request from a curl command line. Options Whimbrel does not \
use are listed as `ignored`. Literal secret values are refused: use {{variables}}."
    )]
    async fn import_curl(
        &self,
        Parameters(args): Parameters<CurlArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.import_curl_in(args)))
    }

    #[tool(
        description = "A folder's requests (or the whole collection's) as .http text, each \
with the auth and headers it inherits, as variables. Nothing is written."
    )]
    async fn export_http(
        &self,
        Parameters(args): Parameters<ExportArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.root(&args.collection).and_then(|root| {
            let (text, count) = wbl_engine::interop::export(&root, &args.folder)?;
            Ok(json!({ "requests": count, "text": text }))
        })))
    }
}

#[cfg(test)]
mod tests;
