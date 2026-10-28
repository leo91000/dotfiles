#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
edition = "2024"

[dependencies]
clap = { version = "4.5.48", features = ["derive"] }
reqwest = { version = "0.12.23", default-features = false, features = ["blocking", "json", "rustls-tls"] }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.145"
sha2 = "0.10.9"
thiserror = "2.0.17"
uuid = "1.18.1"
zeroize = "1.8.2"
---

use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Url;
use reqwest::blocking::{Client, Response};
use reqwest::header::{
    AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue, RETRY_AFTER,
};
use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

const WORKSPACE_ID: &str = "3f66defb-254f-4495-99dd-d1ecb6960259";
const KEY_SENTINEL: &str = "__WEWEB_API_KEY__";
const PSQL_EXECUTOR: &str =
    "/home/leoc/.agents/skills/weweb-psql-env-executor/scripts/exec_psql_env.sh";
const MAX_READ_RETRIES: usize = 3;

#[derive(Parser)]
#[command(
    name = "weweb-api",
    bin_name = "weweb-api",
    version,
    about = "Call WeWeb workspace-key public APIs"
)]
struct Cli {
    #[arg(long, value_enum, default_value_t = Environment::Staging, global = true)]
    env: Environment,

    #[arg(long, value_enum, default_value_t = OutputMode::Json, global = true)]
    output: OutputMode,

    #[arg(long, global = true)]
    apply: bool,

    #[arg(long, global = true)]
    allow_prod: bool,

    #[arg(long, global = true)]
    reveal_secrets: bool,

    #[command(subcommand)]
    command: ApiCommand,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Environment {
    Local,
    Staging,
    Preprod,
    Prod,
}

impl Environment {
    fn api_base_url(self) -> &'static str {
        match self {
            Self::Local => "http://localhost:3000",
            Self::Staging => "https://api.weweb-staging.io",
            Self::Preprod => "https://api.weweb-preprod.io",
            Self::Prod => "https://api.weweb.io",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Staging => "staging",
            Self::Preprod => "preprod",
            Self::Prod => "prod",
        }
    }

    fn psql_ssm_port(self) -> Option<&'static str> {
        match self {
            Self::Preprod => Some("8022"),
            Self::Prod => Some("8023"),
            Self::Local | Self::Staging => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OutputMode {
    Json,
    Table,
    Status,
}

#[derive(Subcommand)]
enum ApiCommand {
    Capabilities,
    Request(RequestArgs),
    Deploy(DeployArgs),
    #[command(name = "deploy-last")]
    DeployLast(ProjectArgs),
    Version(VersionArgs),
    Sitemap(TextResourceArgs),
    Robots(TextResourceArgs),
    Metadata(MetadataArgs),
    Figma(FigmaArgs),
}

#[derive(Args)]
struct RequestArgs {
    method: String,
    path: String,

    #[arg(long = "query", value_name = "KEY=VALUE")]
    query: Vec<String>,

    #[arg(long = "header", value_name = "NAME:VALUE")]
    header: Vec<String>,

    #[command(flatten)]
    body: JsonBodyArgs,

    #[arg(long)]
    output_file: Option<PathBuf>,
}

#[derive(Args)]
struct DeployArgs {
    project_id: String,

    #[command(flatten)]
    body: JsonBodyArgs,
}

#[derive(Args)]
struct ProjectArgs {
    project_id: String,
}

#[derive(Args)]
struct VersionArgs {
    #[command(subcommand)]
    command: VersionCommand,
}

#[derive(Subcommand)]
enum VersionCommand {
    Config(VersionTargetArgs),
    Download(VersionDownloadArgs),
}

#[derive(Args)]
struct VersionTargetArgs {
    project_id: String,
    version: String,
}

#[derive(Clone, Copy, ValueEnum)]
enum DownloadKind {
    Generated,
    Raw,
}

#[derive(Args)]
struct VersionDownloadArgs {
    project_id: String,
    version: String,

    #[arg(long, value_enum, default_value_t = DownloadKind::Generated)]
    kind: DownloadKind,

    #[arg(long)]
    output_file: String,

    #[arg(long = "query", value_name = "KEY=VALUE")]
    query: Vec<String>,
}

#[derive(Args)]
struct TextResourceArgs {
    #[command(subcommand)]
    command: TextResourceCommand,
}

#[derive(Subcommand)]
enum TextResourceCommand {
    Get(ProjectArgs),
    Set(TextSetArgs),
}

#[derive(Args)]
struct TextSetArgs {
    project_id: String,

    #[arg(long, conflicts_with_all = ["file", "stdin"])]
    value: Option<String>,

    #[arg(long, conflicts_with_all = ["value", "stdin"])]
    file: Option<PathBuf>,

    #[arg(long, conflicts_with_all = ["value", "file"])]
    stdin: bool,
}

#[derive(Args)]
struct MetadataArgs {
    #[command(subcommand)]
    command: MetadataCommand,
}

#[derive(Subcommand)]
enum MetadataCommand {
    List(ProjectArgs),
    Create(JsonProjectArgs),
    Update(JsonProjectArgs),
    Upsert(JsonProjectArgs),
    Delete(JsonProjectArgs),
}

#[derive(Args)]
struct JsonProjectArgs {
    project_id: String,

    #[command(flatten)]
    body: JsonBodyArgs,
}

#[derive(Args)]
struct FigmaArgs {
    #[command(subcommand)]
    command: FigmaCommand,
}

#[derive(Subcommand)]
enum FigmaCommand {
    #[command(name = "workspace-info")]
    WorkspaceInfo,
    #[command(name = "coded-components")]
    CodedComponents,
    #[command(name = "library-components")]
    LibraryComponents,
    Mappings(FigmaMappingsArgs),
}

#[derive(Args)]
struct FigmaMappingsArgs {
    #[command(subcommand)]
    command: FigmaMappingsCommand,
}

#[derive(Subcommand)]
enum FigmaMappingsCommand {
    List,
    Upsert(JsonBodyArgs),
    Delete(FigmaMappingDeleteArgs),
}

#[derive(Args)]
#[group(required = true, multiple = false)]
struct FigmaMappingDeleteArgs {
    #[arg(long)]
    ww_object_base_id: Option<String>,

    #[arg(long)]
    library_component_id: Option<String>,
}

#[derive(Args, Default)]
struct JsonBodyArgs {
    #[arg(
        long,
        value_name = "JSON_OBJECT",
        help = "Complete JSON object; dotted flags such as --params.env VALUE are also accepted",
        conflicts_with_all = ["file", "stdin"]
    )]
    params: Option<String>,

    #[arg(long, hide = true, value_name = "PATH=VALUE", conflicts_with_all = ["file", "stdin"])]
    param_entry: Vec<String>,

    #[arg(long, conflicts_with_all = ["params", "param_entry", "stdin"])]
    file: Option<PathBuf>,

    #[arg(long, conflicts_with_all = ["params", "param_entry", "file"])]
    stdin: bool,
}

#[derive(Debug, Error)]
enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Safety(String),
    #[error("{0}")]
    Authentication(String),
    #[error("{0}")]
    Network(String),
    #[error("HTTP {0}")]
    Http4xx(u16),
    #[error("HTTP {0}")]
    Http5xx(u16),
}

impl AppError {
    fn exit_code(&self) -> u8 {
        match self {
            Self::Validation(_) => 2,
            Self::Safety(_) => 3,
            Self::Authentication(_) => 4,
            Self::Network(_) => 5,
            Self::Http4xx(_) => 40,
            Self::Http5xx(_) => 50,
        }
    }
}

type Result<T> = std::result::Result<T, AppError>;

#[derive(Clone)]
struct RequestSpec {
    method: Method,
    path: String,
    query: Vec<(String, String)>,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Capability {
    name: &'static str,
    method: &'static str,
    path: &'static str,
    mutation: bool,
    parameters: &'static [&'static str],
    response: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadMetadata {
    status: u16,
    path: String,
    size: u64,
    content_type: Option<String>,
    sha256: String,
}

fn main() -> ExitCode {
    let args = match normalize_param_args(std::env::args_os()) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(error.exit_code());
        }
    };
    let cli = Cli::parse_from(args);
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(error.exit_code())
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    if matches!(cli.command, ApiCommand::Capabilities) {
        return emit_value(
            serde_json::to_value(capabilities())
                .map_err(|_| AppError::Validation("Failed to serialize capabilities".to_owned()))?,
            cli.output,
        );
    }

    let action = command_to_action(&cli.command)?;
    ensure_mutation_allowed(&action.spec.method, cli.env, cli.apply, cli.allow_prod)?;
    let api_key = load_api_key(cli.env)?;
    let client = build_client()?;

    if let Some(output_file) = action.output_file {
        return execute_download(
            &client,
            cli.env,
            api_key.as_str(),
            &action.spec,
            &output_file,
            cli.output,
            cli.reveal_secrets,
        );
    }

    let response = send_with_retries(&client, cli.env, api_key.as_str(), &action.spec)?;
    emit_http_response(response, cli.output, cli.reveal_secrets, api_key.as_str())
}

struct CommandAction {
    spec: RequestSpec,
    output_file: Option<String>,
}

fn command_to_action(command: &ApiCommand) -> Result<CommandAction> {
    match command {
        ApiCommand::Capabilities => Err(AppError::Validation(
            "Capabilities does not issue an HTTP request".to_owned(),
        )),
        ApiCommand::Request(args) => raw_request_action(args),
        ApiCommand::Deploy(args) => {
            let body = read_optional_json_body(&args.body)?.unwrap_or_else(|| b"{}".to_vec());
            action(
                Method::POST,
                &project_path(&args.project_id, "deploy")?,
                Vec::new(),
                Some(body),
            )
        }
        ApiCommand::DeployLast(args) => action(
            Method::GET,
            &project_path(&args.project_id, "deploy/last")?,
            Vec::new(),
            None,
        ),
        ApiCommand::Version(args) => match &args.command {
            VersionCommand::Config(args) => action(
                Method::GET,
                &version_path(&args.project_id, &args.version, "config")?,
                Vec::new(),
                None,
            ),
            VersionCommand::Download(args) => {
                let suffix = match args.kind {
                    DownloadKind::Generated => "download",
                    DownloadKind::Raw => "download/raw",
                };
                let mut result = action(
                    Method::GET,
                    &version_path(&args.project_id, &args.version, suffix)?,
                    parse_query_pairs(&args.query)?,
                    None,
                )?;
                result.output_file = Some(args.output_file.clone());
                Ok(result)
            }
        },
        ApiCommand::Sitemap(args) => text_resource_action(args, "sitemap_xml", "sitemapXml"),
        ApiCommand::Robots(args) => text_resource_action(args, "robots_txt", "robotsTxt"),
        ApiCommand::Metadata(args) => metadata_action(args),
        ApiCommand::Figma(args) => figma_action(args),
    }
}

fn raw_request_action(args: &RequestArgs) -> Result<CommandAction> {
    let method = Method::from_bytes(args.method.as_bytes())
        .map_err(|_| AppError::Validation("Invalid HTTP method".to_owned()))?;
    validate_public_path(&args.path)?;
    let headers = parse_headers(&args.header)?;
    let body = read_optional_json_body(&args.body)?;
    Ok(CommandAction {
        spec: RequestSpec {
            method,
            path: args.path.clone(),
            query: parse_query_pairs(&args.query)?,
            headers,
            body,
        },
        output_file: args
            .output_file
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
    })
}

fn action(
    method: Method,
    path: &str,
    query: Vec<(String, String)>,
    body: Option<Vec<u8>>,
) -> Result<CommandAction> {
    validate_public_path(path)?;
    Ok(CommandAction {
        spec: RequestSpec {
            method,
            path: path.to_owned(),
            query,
            headers: HeaderMap::new(),
            body,
        },
        output_file: None,
    })
}

fn text_resource_action(
    args: &TextResourceArgs,
    route: &str,
    body_key: &str,
) -> Result<CommandAction> {
    match &args.command {
        TextResourceCommand::Get(args) => action(
            Method::GET,
            &project_path(&args.project_id, route)?,
            Vec::new(),
            None,
        ),
        TextResourceCommand::Set(args) => {
            let value = read_required_text(args)?;
            let body = serde_json::to_vec(&json!({ (body_key): value }))
                .map_err(|_| AppError::Validation("Failed to serialize request body".to_owned()))?;
            action(
                Method::POST,
                &project_path(&args.project_id, route)?,
                Vec::new(),
                Some(body),
            )
        }
    }
}

fn metadata_action(args: &MetadataArgs) -> Result<CommandAction> {
    let (method, project_id, body) = match &args.command {
        MetadataCommand::List(args) => (Method::GET, &args.project_id, None),
        MetadataCommand::Create(args) => (
            Method::POST,
            &args.project_id,
            Some(read_required_json_body(&args.body)?),
        ),
        MetadataCommand::Update(args) => (
            Method::PATCH,
            &args.project_id,
            Some(read_required_json_body(&args.body)?),
        ),
        MetadataCommand::Upsert(args) => (
            Method::PUT,
            &args.project_id,
            Some(read_required_json_body(&args.body)?),
        ),
        MetadataCommand::Delete(args) => (
            Method::DELETE,
            &args.project_id,
            Some(read_required_json_body(&args.body)?),
        ),
    };
    action(
        method,
        &project_path(project_id, "pages/metadata")?,
        Vec::new(),
        body,
    )
}

fn figma_action(args: &FigmaArgs) -> Result<CommandAction> {
    match &args.command {
        FigmaCommand::WorkspaceInfo => action(
            Method::GET,
            "/public/v1/figma/workspace-info",
            Vec::new(),
            None,
        ),
        FigmaCommand::CodedComponents => action(
            Method::GET,
            "/public/v1/figma/coded-components",
            Vec::new(),
            None,
        ),
        FigmaCommand::LibraryComponents => action(
            Method::GET,
            "/public/v1/figma/library-components",
            Vec::new(),
            None,
        ),
        FigmaCommand::Mappings(args) => match &args.command {
            FigmaMappingsCommand::List => {
                action(Method::GET, "/public/v1/figma/mappings", Vec::new(), None)
            }
            FigmaMappingsCommand::Upsert(body) => action(
                Method::POST,
                "/public/v1/figma/mappings",
                Vec::new(),
                Some(read_required_json_body(body)?),
            ),
            FigmaMappingsCommand::Delete(args) => {
                let (key, value) = if let Some(value) = &args.ww_object_base_id {
                    validate_uuid(value, "ww-object-base-id")?;
                    ("wwObjectBaseId", value.clone())
                } else if let Some(value) = &args.library_component_id {
                    validate_uuid(value, "library-component-id")?;
                    ("libraryComponentId", value.clone())
                } else {
                    return Err(AppError::Validation(
                        "Provide exactly one mapping component id".to_owned(),
                    ));
                };
                action(
                    Method::DELETE,
                    "/public/v1/figma/mappings",
                    vec![(key.to_owned(), value)],
                    None,
                )
            }
        },
    }
}

fn project_path(project_id: &str, suffix: &str) -> Result<String> {
    validate_uuid(project_id, "project-id")?;
    Ok(format!(
        "/public/v1/workspaces/{WORKSPACE_ID}/projects/{project_id}/{suffix}"
    ))
}

fn version_path(project_id: &str, version: &str, suffix: &str) -> Result<String> {
    validate_version(version)?;
    Ok(format!(
        "{}/versions/{version}/{suffix}",
        project_path(project_id, "")?.trim_end_matches('/')
    ))
}

fn validate_uuid(value: &str, label: &str) -> Result<()> {
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| AppError::Validation(format!("Invalid {label}: expected UUID")))
}

fn validate_version(value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character))
    {
        return Err(AppError::Validation(
            "Invalid version: use only letters, digits, dot, dash, or underscore".to_owned(),
        ));
    }
    Ok(())
}

fn validate_public_path(path: &str) -> Result<()> {
    let lower = path.to_ascii_lowercase();
    if !path.starts_with("/public/v1/") {
        return Err(AppError::Safety(
            "Only relative /public/v1/ paths are allowed".to_owned(),
        ));
    }
    if path.starts_with("//")
        || path.contains('?')
        || path.contains('#')
        || lower.contains("://")
        || lower.contains("%2e")
        || path.split('/').any(|segment| matches!(segment, "." | ".."))
    {
        return Err(AppError::Safety(
            "Unsafe path; use a relative normalized path and --query parameters".to_owned(),
        ));
    }
    Ok(())
}

fn ensure_mutation_allowed(
    method: &Method,
    env: Environment,
    apply: bool,
    allow_prod: bool,
) -> Result<()> {
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return Ok(());
    }
    if !apply {
        return Err(AppError::Safety(format!(
            "{} is a mutation; pass --apply",
            method.as_str()
        )));
    }
    if env == Environment::Prod && !allow_prod {
        return Err(AppError::Safety(
            "Production mutations require --apply --allow-prod".to_owned(),
        ));
    }
    Ok(())
}

fn parse_query_pairs(values: &[String]) -> Result<Vec<(String, String)>> {
    values
        .iter()
        .map(|value| {
            let (key, value) = value.split_once('=').ok_or_else(|| {
                AppError::Validation("Query parameters must use KEY=VALUE".to_owned())
            })?;
            if key.is_empty() {
                return Err(AppError::Validation(
                    "Query parameter keys cannot be empty".to_owned(),
                ));
            }
            Ok((key.to_owned(), value.to_owned()))
        })
        .collect()
}

fn parse_headers(values: &[String]) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    for value in values {
        let (name, value) = value
            .split_once(':')
            .ok_or_else(|| AppError::Validation("Headers must use NAME:VALUE".to_owned()))?;
        let normalized = name.trim().to_ascii_lowercase();
        if matches!(
            normalized.as_str(),
            "authorization" | "proxy-authorization" | "host"
        ) {
            return Err(AppError::Safety(format!(
                "Header {name} is reserved by the CLI"
            )));
        }
        let name = HeaderName::from_bytes(normalized.as_bytes())
            .map_err(|_| AppError::Validation("Invalid header name".to_owned()))?;
        let value = HeaderValue::from_str(value.trim())
            .map_err(|_| AppError::Validation("Invalid header value".to_owned()))?;
        headers.insert(name, value);
    }
    Ok(headers)
}

fn read_optional_json_body(args: &JsonBodyArgs) -> Result<Option<Vec<u8>>> {
    if args.params.is_some() || !args.param_entry.is_empty() {
        return build_params_body(args).map(Some);
    }

    let content = if let Some(path) = &args.file {
        Some(fs::read_to_string(path).map_err(|_| {
            AppError::Validation(format!("Failed to read body file {}", path.display()))
        })?)
    } else if args.stdin {
        Some(read_stdin()?)
    } else {
        None
    };

    content
        .map(|content| normalize_json_body(&content))
        .transpose()
}

fn read_required_json_body(args: &JsonBodyArgs) -> Result<Vec<u8>> {
    read_optional_json_body(args)?.ok_or_else(|| {
        AppError::Validation(
            "Provide --params, one or more --params.*, --file, or --stdin".to_owned(),
        )
    })
}

fn normalize_param_args<I>(args: I) -> Result<Vec<OsString>>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let mut normalized = Vec::new();
    if let Some(program) = args.next() {
        normalized.push(program);
    }

    while let Some(arg) = args.next() {
        let value = arg
            .to_str()
            .ok_or_else(|| AppError::Validation("CLI arguments must be valid UTF-8".to_owned()))?;
        let Some(param) = value.strip_prefix("--params.") else {
            normalized.push(arg);
            continue;
        };
        if param.is_empty() {
            return Err(AppError::Validation(
                "Dotted parameter paths cannot be empty".to_owned(),
            ));
        }

        let entry = if let Some((path, value)) = param.split_once('=') {
            if path.is_empty() {
                return Err(AppError::Validation(
                    "Dotted parameter paths cannot be empty".to_owned(),
                ));
            }
            format!("{path}={value}")
        } else {
            let value = args.next().ok_or_else(|| {
                AppError::Validation(format!("Missing value for --params.{param}"))
            })?;
            let value = value.to_str().ok_or_else(|| {
                AppError::Validation("Parameter values must be valid UTF-8".to_owned())
            })?;
            format!("{param}={value}")
        };
        normalized.push(OsString::from("--param-entry"));
        normalized.push(OsString::from(entry));
    }

    Ok(normalized)
}

fn build_params_body(args: &JsonBodyArgs) -> Result<Vec<u8>> {
    let mut body = if let Some(params) = &args.params {
        let value: Value = serde_json::from_str(params).map_err(|_| {
            AppError::Validation("--params must contain a valid JSON object".to_owned())
        })?;
        if !value.is_object() {
            return Err(AppError::Validation(
                "--params must contain a JSON object".to_owned(),
            ));
        }
        value
    } else {
        Value::Object(Map::new())
    };

    for entry in &args.param_entry {
        let (path, raw_value) = entry
            .split_once('=')
            .ok_or_else(|| AppError::Validation("Invalid dotted parameter encoding".to_owned()))?;
        let value =
            serde_json::from_str(raw_value).unwrap_or_else(|_| Value::String(raw_value.to_owned()));
        insert_param_path(&mut body, path, value)?;
    }

    serde_json::to_vec(&body)
        .map_err(|_| AppError::Validation("Failed to serialize request parameters".to_owned()))
}

fn insert_param_path(root: &mut Value, path: &str, value: Value) -> Result<()> {
    let segments: Vec<&str> = path.split('.').collect();
    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| segment.is_empty() || segment.chars().any(char::is_control))
    {
        return Err(AppError::Validation(
            "Dotted parameter paths must contain non-empty segments".to_owned(),
        ));
    }

    let mut current = root;
    for segment in &segments[..segments.len() - 1] {
        let object = current.as_object_mut().ok_or_else(|| {
            AppError::Validation(format!(
                "Cannot create nested parameter below non-object path {segment}"
            ))
        })?;
        current = object
            .entry((*segment).to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        if !current.is_object() {
            return Err(AppError::Validation(format!(
                "Cannot create nested parameter below non-object path {segment}"
            )));
        }
    }

    current
        .as_object_mut()
        .ok_or_else(|| AppError::Validation("Request parameters must be an object".to_owned()))?
        .insert(segments[segments.len() - 1].to_owned(), value);
    Ok(())
}

fn normalize_json_body(content: &str) -> Result<Vec<u8>> {
    let value: Value = serde_json::from_str(content)
        .map_err(|_| AppError::Validation("Request body must be valid JSON".to_owned()))?;
    serde_json::to_vec(&value)
        .map_err(|_| AppError::Validation("Failed to serialize request body".to_owned()))
}

fn read_required_text(args: &TextSetArgs) -> Result<String> {
    if let Some(value) = &args.value {
        return Ok(value.clone());
    }
    if let Some(path) = &args.file {
        return fs::read_to_string(path).map_err(|_| {
            AppError::Validation(format!("Failed to read text file {}", path.display()))
        });
    }
    if args.stdin {
        return read_stdin();
    }
    Err(AppError::Validation(
        "Provide one of --value, --file, or --stdin".to_owned(),
    ))
}

fn read_stdin() -> Result<String> {
    if io::stdin().is_terminal() {
        return Err(AppError::Validation(
            "--stdin requires piped input".to_owned(),
        ));
    }
    let mut content = String::new();
    io::stdin()
        .read_to_string(&mut content)
        .map_err(|_| AppError::Validation("Failed to read stdin".to_owned()))?;
    Ok(content)
}

fn load_api_key(env: Environment) -> Result<Zeroizing<String>> {
    if let Ok(value) = std::env::var("WEWEB_API_KEY") {
        let value = Zeroizing::new(value);
        if !value.trim().is_empty() {
            validate_api_key(value.as_str())?;
            return Ok(value);
        }
    }

    let sql = format!(
        "COPY (SELECT '{KEY_SENTINEL}' || (\"selfHostSettings\"->>'privateKey') FROM organizations WHERE id = '{WORKSPACE_ID}' AND NULLIF(\"selfHostSettings\"->>'privateKey', '') IS NOT NULL) TO STDOUT;"
    );
    let mut command = Command::new(PSQL_EXECUTOR);
    command.args(["--env", env.as_str(), "--service", "back"]);
    if let Some(port) = env.psql_ssm_port() {
        command.args(["--ssm-port", port]);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| AppError::Authentication("Failed to start WeWeb PSQL executor".to_owned()))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| AppError::Authentication("Failed to open PSQL stdin".to_owned()))?;
    stdin
        .write_all(sql.as_bytes())
        .map_err(|_| AppError::Authentication("Failed to send key lookup query".to_owned()))?;
    drop(stdin);
    let mut output = child.wait_with_output().map_err(|_| {
        AppError::Authentication("Failed to wait for WeWeb PSQL executor".to_owned())
    })?;
    if !output.status.success() {
        output.stdout.zeroize();
        output.stderr.zeroize();
        return Err(AppError::Authentication(format!(
            "Workspace key lookup failed for {}",
            env.as_str()
        )));
    }

    let result = parse_key_output(&output.stdout);
    output.stdout.zeroize();
    output.stderr.zeroize();
    result
}

fn parse_key_output(bytes: &[u8]) -> Result<Zeroizing<String>> {
    let output = std::str::from_utf8(bytes)
        .map_err(|_| AppError::Authentication("Invalid key lookup output".to_owned()))?;
    let matches: Vec<&str> = output
        .lines()
        .filter_map(|line| line.trim().strip_prefix(KEY_SENTINEL))
        .collect();
    if matches.len() != 1 || matches[0].is_empty() {
        return Err(AppError::Authentication(
            "Expected exactly one non-empty workspace API key".to_owned(),
        ));
    }
    let key = Zeroizing::new(matches[0].to_owned());
    validate_api_key(key.as_str())?;
    Ok(key)
}

fn validate_api_key(key: &str) -> Result<()> {
    let prefix = "WW-PRIVATE-";
    let suffix = key
        .strip_prefix(prefix)
        .or_else(|| key.strip_prefix(&prefix.to_ascii_lowercase()))
        .ok_or_else(|| AppError::Authentication("Invalid workspace API key format".to_owned()))?;
    Uuid::parse_str(suffix)
        .map(|_| ())
        .map_err(|_| AppError::Authentication("Invalid workspace API key format".to_owned()))
}

fn build_client() -> Result<Client> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|_| AppError::Network("Failed to build HTTP client".to_owned()))
}

fn build_url(env: Environment, spec: &RequestSpec) -> Result<Url> {
    validate_public_path(&spec.path)?;
    let mut url = Url::parse(env.api_base_url())
        .map_err(|_| AppError::Validation("Invalid configured API origin".to_owned()))?
        .join(&spec.path)
        .map_err(|_| AppError::Validation("Invalid request path".to_owned()))?;
    if url.origin().ascii_serialization() != env.api_base_url() {
        return Err(AppError::Safety("Cross-origin request refused".to_owned()));
    }
    url.query_pairs_mut().extend_pairs(&spec.query);
    Ok(url)
}

fn send_with_retries(
    client: &Client,
    env: Environment,
    api_key: &str,
    spec: &RequestSpec,
) -> Result<Response> {
    let url = build_url(env, spec)?;
    let can_retry = matches!(spec.method, Method::GET | Method::HEAD);

    for attempt in 0..=MAX_READ_RETRIES {
        let mut authorization = HeaderValue::from_str(&format!("Bearer {api_key}"))
            .map_err(|_| AppError::Authentication("Invalid workspace API key".to_owned()))?;
        authorization.set_sensitive(true);

        let mut request = client
            .request(spec.method.clone(), url.clone())
            .header(AUTHORIZATION, authorization)
            .headers(spec.headers.clone());
        if let Some(body) = &spec.body {
            request = request
                .header(CONTENT_TYPE, "application/json")
                .body(body.clone());
        }

        let response = request
            .send()
            .map_err(|error| AppError::Network(safe_reqwest_error(error)))?;
        if !can_retry || attempt == MAX_READ_RETRIES || !is_retryable_status(response.status()) {
            return Ok(response);
        }

        let delay = retry_delay(&response, attempt);
        drop(response);
        thread::sleep(delay);
    }

    Err(AppError::Network("HTTP retry loop exhausted".to_owned()))
}

fn safe_reqwest_error(error: reqwest::Error) -> String {
    let kind = if error.is_timeout() {
        "timed out"
    } else if error.is_connect() {
        "could not connect"
    } else if error.is_request() {
        "request construction failed"
    } else {
        "transport failed"
    };
    format!("HTTP request {kind}")
}

fn is_retryable_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

fn retry_delay(response: &Response, attempt: usize) -> Duration {
    if let Some(seconds) = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
    {
        return Duration::from_secs(seconds.min(30));
    }
    Duration::from_millis(350 * 2_u64.pow(attempt as u32))
}

fn emit_http_response(
    response: Response,
    output: OutputMode,
    reveal_secrets: bool,
    api_key: &str,
) -> Result<()> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = response
        .bytes()
        .map_err(|_| AppError::Network("Failed to read HTTP response".to_owned()))?;

    if output == OutputMode::Status {
        println!("{}", status.as_u16());
    } else {
        let mut value = if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
            value
        } else {
            let body = std::str::from_utf8(&bytes).map_err(|_| {
                AppError::Validation(
                    "Binary response requires --output-file or a typed download command".to_owned(),
                )
            })?;
            json!({
                "status": status.as_u16(),
                "contentType": content_type,
                "body": body,
            })
        };
        if !reveal_secrets {
            redact_value(&mut value, api_key);
        }
        emit_value(value, output)?;
    }

    status_result(status)
}

fn execute_download(
    client: &Client,
    env: Environment,
    api_key: &str,
    spec: &RequestSpec,
    output_file: &str,
    output: OutputMode,
    reveal_secrets: bool,
) -> Result<()> {
    let response = send_with_retries(client, env, api_key, spec)?;
    let status = response.status();
    if !status.is_success() {
        return emit_http_response(response, output, reveal_secrets, api_key);
    }
    if output_file == "-" {
        let mut stdout = io::stdout().lock();
        let mut response = response;
        io::copy(&mut response, &mut stdout)
            .map_err(|_| AppError::Network("Failed to stream response to stdout".to_owned()))?;
        return Ok(());
    }

    let metadata = write_response_atomically(response, Path::new(output_file))?;
    emit_value(
        serde_json::to_value(metadata)
            .map_err(|_| AppError::Validation("Failed to serialize download result".to_owned()))?,
        output,
    )
}

fn write_response_atomically(mut response: Response, path: &Path) -> Result<DownloadMetadata> {
    if path.exists() {
        return Err(AppError::Safety(format!(
            "Output file {} already exists",
            path.display()
        )));
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        return Err(AppError::Validation(format!(
            "Output directory {} does not exist",
            parent.display()
        )));
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::Validation("Invalid output filename".to_owned()))?;
    let temp_path = parent.join(format!(".{file_name}.part.{}", std::process::id()));
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let status = response.status().as_u16();

    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|_| AppError::Validation("Failed to create temporary download".to_owned()))?;
        let mut hasher = Sha256::new();
        let mut size = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = response
                .read(&mut buffer)
                .map_err(|_| AppError::Network("Failed while downloading response".to_owned()))?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])
                .map_err(|_| AppError::Validation("Failed to write download".to_owned()))?;
            hasher.update(&buffer[..read]);
            size += read as u64;
        }
        file.sync_all()
            .map_err(|_| AppError::Validation("Failed to sync download".to_owned()))?;
        drop(file);
        fs::hard_link(&temp_path, path).map_err(|_| {
            AppError::Safety("Failed to publish download without overwrite".to_owned())
        })?;
        fs::remove_file(&temp_path)
            .map_err(|_| AppError::Validation("Failed to remove temporary download".to_owned()))?;
        Ok(DownloadMetadata {
            status,
            path: path.display().to_string(),
            size,
            content_type,
            sha256: format!("{:x}", hasher.finalize()),
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn status_result(status: StatusCode) -> Result<()> {
    if status.is_success() {
        Ok(())
    } else if status.is_client_error() {
        Err(AppError::Http4xx(status.as_u16()))
    } else if status.is_server_error() {
        Err(AppError::Http5xx(status.as_u16()))
    } else {
        Err(AppError::Network(format!(
            "Unexpected HTTP status {}",
            status.as_u16()
        )))
    }
}

fn emit_value(value: Value, output: OutputMode) -> Result<()> {
    match output {
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).map_err(|_| {
                    AppError::Validation("Failed to serialize JSON output".to_owned())
                })?
            );
        }
        OutputMode::Table => print_table(&value),
        OutputMode::Status => println!("0"),
    }
    Ok(())
}

fn print_table(value: &Value) {
    match value {
        Value::Array(rows) if rows.iter().all(Value::is_object) => {
            let columns: BTreeSet<&str> = rows
                .iter()
                .filter_map(Value::as_object)
                .flat_map(|row| row.keys().map(String::as_str))
                .collect();
            println!("{}", columns.iter().copied().collect::<Vec<_>>().join("\t"));
            for row in rows.iter().filter_map(Value::as_object) {
                let values = columns
                    .iter()
                    .map(|column| table_cell(row.get(*column).unwrap_or(&Value::Null)))
                    .collect::<Vec<_>>();
                println!("{}", values.join("\t"));
            }
        }
        Value::Object(row) => {
            for (key, value) in row {
                println!("{key}\t{}", table_cell(value));
            }
        }
        _ => println!("{}", table_cell(value)),
    }
}

fn table_cell(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.replace(['\t', '\n', '\r'], " "),
        _ => value.to_string(),
    }
}

fn redact_value(value: &mut Value, api_key: &str) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if is_sensitive_key(key) {
                    *value = Value::String("[REDACTED]".to_owned());
                } else {
                    redact_value(value, api_key);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                redact_value(value, api_key);
            }
        }
        Value::String(value) => *value = redact_text(value, api_key),
        _ => {}
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    ["token", "secret", "password", "privatekey", "authorization"]
        .iter()
        .any(|part| normalized.contains(part))
}

fn redact_text(value: &str, api_key: &str) -> String {
    let mut result = redact_bearer_tokens(value).replace(api_key, "[REDACTED]");
    for prefix in ["WW-PRIVATE-", "ww-private-"] {
        while let Some(start) = result.find(prefix) {
            let suffix_start = start + prefix.len();
            let suffix_len = result[suffix_start..]
                .chars()
                .take_while(|character| character.is_ascii_hexdigit() || *character == '-')
                .map(char::len_utf8)
                .sum::<usize>();
            if suffix_len == 0 {
                break;
            }
            result.replace_range(start..suffix_start + suffix_len, "[REDACTED]");
        }
    }
    result
}

fn redact_bearer_tokens(value: &str) -> String {
    let mut remaining = value;
    let mut redacted = String::with_capacity(value.len());
    loop {
        let lowercase = remaining.to_ascii_lowercase();
        let Some(start) = lowercase.find("bearer ") else {
            redacted.push_str(remaining);
            break;
        };
        redacted.push_str(&remaining[..start]);
        let token_start = start + "bearer ".len();
        let token_len = remaining[token_start..]
            .chars()
            .take_while(|character| {
                !character.is_whitespace() && !matches!(character, '"' | '\'' | '<' | '>' | ',')
            })
            .map(char::len_utf8)
            .sum::<usize>();
        redacted.push_str("Bearer [REDACTED]");
        remaining = &remaining[token_start + token_len..];
    }
    redacted
}

fn capabilities() -> Vec<Capability> {
    vec![
        capability(
            "deploy",
            "POST",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/deploy",
            true,
            &["projectId", "body"],
            "json",
        ),
        capability(
            "deploy-last",
            "GET",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/deploy/last",
            false,
            &["projectId"],
            "json",
        ),
        capability(
            "version-config",
            "GET",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/versions/{version}/config",
            false,
            &["projectId", "version"],
            "json",
        ),
        capability(
            "version-download-generated",
            "GET",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/versions/{version}/download",
            false,
            &["projectId", "version", "outputFile"],
            "binary",
        ),
        capability(
            "version-download-raw",
            "GET",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/versions/{version}/download/raw",
            false,
            &["projectId", "version", "outputFile"],
            "binary",
        ),
        capability(
            "sitemap-get",
            "GET",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/sitemap_xml",
            false,
            &["projectId"],
            "text",
        ),
        capability(
            "sitemap-set",
            "POST",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/sitemap_xml",
            true,
            &["projectId", "sitemapXml"],
            "json",
        ),
        capability(
            "robots-get",
            "GET",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/robots_txt",
            false,
            &["projectId"],
            "text",
        ),
        capability(
            "robots-set",
            "POST",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/robots_txt",
            true,
            &["projectId", "robotsTxt"],
            "json",
        ),
        capability(
            "metadata-list",
            "GET",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/pages/metadata",
            false,
            &["projectId"],
            "json",
        ),
        capability(
            "metadata-create",
            "POST",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/pages/metadata",
            true,
            &["projectId", "body"],
            "json",
        ),
        capability(
            "metadata-update",
            "PATCH",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/pages/metadata",
            true,
            &["projectId", "body"],
            "json",
        ),
        capability(
            "metadata-upsert",
            "PUT",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/pages/metadata",
            true,
            &["projectId", "body"],
            "json",
        ),
        capability(
            "metadata-delete",
            "DELETE",
            "/public/v1/workspaces/{workspaceId}/projects/{projectId}/pages/metadata",
            true,
            &["projectId", "body"],
            "json",
        ),
        capability(
            "figma-workspace-info",
            "GET",
            "/public/v1/figma/workspace-info",
            false,
            &[],
            "json",
        ),
        capability(
            "figma-coded-components",
            "GET",
            "/public/v1/figma/coded-components",
            false,
            &[],
            "json",
        ),
        capability(
            "figma-library-components",
            "GET",
            "/public/v1/figma/library-components",
            false,
            &[],
            "json",
        ),
        capability(
            "figma-mappings-list",
            "GET",
            "/public/v1/figma/mappings",
            false,
            &[],
            "json",
        ),
        capability(
            "figma-mappings-upsert",
            "POST",
            "/public/v1/figma/mappings",
            true,
            &["body"],
            "json",
        ),
        capability(
            "figma-mappings-delete",
            "DELETE",
            "/public/v1/figma/mappings",
            true,
            &["wwObjectBaseId|libraryComponentId"],
            "json",
        ),
    ]
}

fn capability(
    name: &'static str,
    method: &'static str,
    path: &'static str,
    mutation: bool,
    parameters: &'static [&'static str],
    response: &'static str,
) -> Capability {
    Capability {
        name,
        method,
        path,
        mutation,
        parameters,
        response,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: &str = "WW-PRIVATE-123E4567-E89B-12D3-A456-426614174000";

    #[test]
    fn maps_all_environment_origins() {
        assert_eq!(Environment::Local.api_base_url(), "http://localhost:3000");
        assert_eq!(
            Environment::Staging.api_base_url(),
            "https://api.weweb-staging.io"
        );
        assert_eq!(
            Environment::Preprod.api_base_url(),
            "https://api.weweb-preprod.io"
        );
        assert_eq!(Environment::Prod.api_base_url(), "https://api.weweb.io");
        assert_eq!(Environment::Local.psql_ssm_port(), None);
        assert_eq!(Environment::Staging.psql_ssm_port(), None);
        assert_eq!(Environment::Preprod.psql_ssm_port(), Some("8022"));
        assert_eq!(Environment::Prod.psql_ssm_port(), Some("8023"));
    }

    #[test]
    fn rejects_non_public_and_ambiguous_paths() {
        for path in [
            "https://evil.example/public/v1/x",
            "//evil.example/public/v1/x",
            "/v1/admin",
            "/public/v1/../admin",
            "/public/v1/%2e%2e/admin",
            "/public/v1/test?secret=value",
        ] {
            assert!(validate_public_path(path).is_err(), "accepted {path}");
        }
        assert!(validate_public_path("/public/v1/figma/workspace-info").is_ok());
    }

    #[test]
    fn enforces_mutation_and_production_gates() {
        assert!(ensure_mutation_allowed(&Method::GET, Environment::Prod, false, false).is_ok());
        assert!(
            ensure_mutation_allowed(&Method::POST, Environment::Staging, false, false).is_err()
        );
        assert!(ensure_mutation_allowed(&Method::POST, Environment::Staging, true, false).is_ok());
        assert!(ensure_mutation_allowed(&Method::DELETE, Environment::Prod, true, false).is_err());
        assert!(ensure_mutation_allowed(&Method::DELETE, Environment::Prod, true, true).is_ok());
    }

    #[test]
    fn parses_exactly_one_sentinel_key() {
        let parsed =
            parse_key_output(format!("noise\n{KEY_SENTINEL}{TEST_KEY}\n").as_bytes()).unwrap();
        assert_eq!(parsed.as_str(), TEST_KEY);
        assert!(parse_key_output(b"noise only\n").is_err());
        assert!(parse_key_output(format!("{KEY_SENTINEL}\n").as_bytes()).is_err());
        assert!(
            parse_key_output(
                format!("{KEY_SENTINEL}{TEST_KEY}\n{KEY_SENTINEL}{TEST_KEY}\n").as_bytes()
            )
            .is_err()
        );
    }

    #[test]
    fn redacts_sensitive_keys_and_values() {
        let mut value = json!({
            "privateKey": TEST_KEY,
            "nested": {
                "message": format!("Bearer {TEST_KEY}"),
                "safe": "visible"
            }
        });
        redact_value(&mut value, TEST_KEY);
        assert_eq!(value["privateKey"], "[REDACTED]");
        assert!(!value.to_string().contains(TEST_KEY));
        assert_eq!(value["nested"]["safe"], "visible");
        assert_eq!(
            redact_text("failure for bearer another-token", TEST_KEY),
            "failure for Bearer [REDACTED]"
        );
    }

    #[test]
    fn builds_canonical_project_paths() {
        let project_id = "123e4567-e89b-12d3-a456-426614174000";
        assert_eq!(
            project_path(project_id, "deploy").unwrap(),
            format!("/public/v1/workspaces/{WORKSPACE_ID}/projects/{project_id}/deploy")
        );
        assert!(project_path("not-a-uuid", "deploy").is_err());
    }

    #[test]
    fn reserves_authentication_and_host_headers() {
        assert!(parse_headers(&["Authorization: nope".to_owned()]).is_err());
        assert!(parse_headers(&["Proxy-Authorization: nope".to_owned()]).is_err());
        assert!(parse_headers(&["Host: evil.example".to_owned()]).is_err());
        assert!(parse_headers(&["Accept: application/json".to_owned()]).is_ok());
    }

    #[test]
    fn exposes_twenty_current_capabilities() {
        let capabilities = capabilities();
        assert_eq!(capabilities.len(), 20);
        assert!(
            capabilities
                .iter()
                .all(|item| !item.path.contains("/publish"))
        );
    }

    #[test]
    fn uses_the_server_contract_key_for_text_resources() {
        let args = TextResourceArgs {
            command: TextResourceCommand::Set(TextSetArgs {
                project_id: "123e4567-e89b-12d3-a456-426614174000".to_owned(),
                value: Some("content".to_owned()),
                file: None,
                stdin: false,
            }),
        };
        let sitemap = text_resource_action(&args, "sitemap_xml", "sitemapXml").unwrap();
        let body: Value = serde_json::from_slice(sitemap.spec.body.as_ref().unwrap()).unwrap();
        assert_eq!(body, json!({ "sitemapXml": "content" }));
    }

    #[test]
    fn normalizes_dotted_param_flags_for_clap() {
        let args = vec![
            OsString::from("weweb-api"),
            OsString::from("deploy"),
            OsString::from("123e4567-e89b-12d3-a456-426614174000"),
            OsString::from("--params.env"),
            OsString::from("staging"),
            OsString::from("--params.githubEnabled=true"),
        ];
        let normalized = normalize_param_args(args).unwrap();
        let cli = Cli::try_parse_from(normalized).unwrap();
        let ApiCommand::Deploy(deploy) = cli.command else {
            panic!("expected deploy command");
        };
        assert_eq!(
            deploy.body.param_entry,
            ["env=staging", "githubEnabled=true"]
        );
    }

    #[test]
    fn builds_nested_typed_params_and_applies_overrides() {
        let args = JsonBodyArgs {
            params: Some(r#"{"env":"preprod","mapping":{"existing":1}}"#.to_owned()),
            param_entry: vec![
                "env=staging".to_owned(),
                "githubEnabled=true".to_owned(),
                "mapping.tag=button".to_owned(),
                "count=42".to_owned(),
                "optional=null".to_owned(),
            ],
            file: None,
            stdin: false,
        };
        let body: Value = serde_json::from_slice(&build_params_body(&args).unwrap()).unwrap();
        assert_eq!(
            body,
            json!({
                "env": "staging",
                "githubEnabled": true,
                "mapping": { "existing": 1, "tag": "button" },
                "count": 42,
                "optional": null
            })
        );
    }

    #[test]
    fn rejects_nested_params_below_scalar_values() {
        let args = JsonBodyArgs {
            params: None,
            param_entry: vec!["mapping=text".to_owned(), "mapping.tag=button".to_owned()],
            file: None,
            stdin: false,
        };
        assert!(build_params_body(&args).is_err());
    }
}
