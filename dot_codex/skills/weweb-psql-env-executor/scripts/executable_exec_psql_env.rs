#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
edition = "2024"

[dependencies]
anyhow = "1.0.100"
blake3 = "1.8.2"
clap = { version = "4.5.48", features = ["derive"] }
fs2 = "0.4.3"
libc = "0.2.177"
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.145"
---

use anyhow::{bail, Context, Result};
use clap::Parser;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const VAULT: &str = "WeWeb";
const STATE_DIR: &str = "/tmp/weweb-psql-env-executor";
const IDLE_TTL_SECS: i64 = 300;
const OP_CACHE_TTL_SECS: i64 = 300;
const TOUCH_INTERVAL_SECS: u64 = 30;
const TUNNEL_READY_TIMEOUT_SECS: u64 = 12;
const DEFAULT_LOCAL_DOCKER_DIR: &str = "/home/leoc/projects/weweb/weweb-docker";
const DEFAULT_PROD_SSM_TARGET: &str = "i-00128b669966cf841";
const DEFAULT_PROD_SSM_SSH_LOCAL_PORT: u16 = 8022;
static TEMPFILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Parser, Debug)]
#[command(name = "exec_psql_env.sh", disable_version_flag = true)]
struct PublicCli {
    #[arg(long)]
    env: String,
    #[arg(long)]
    service: Option<String>,
    #[arg(long)]
    sql: Option<String>,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long, default_value = "my.1password.eu")]
    op_account: String,
    #[arg(long, default_value_t = 15432)]
    local_port: u16,
    #[arg(long)]
    postgres_port: Option<u16>,
    #[arg(long)]
    aws_profile: Option<String>,
    #[arg(long)]
    ssm_target: Option<String>,
    #[arg(long, default_value_t = DEFAULT_PROD_SSM_SSH_LOCAL_PORT)]
    ssm_port: u16,
    #[arg(long, default_value_t = OP_CACHE_TTL_SECS)]
    op_cache_ttl_seconds: i64,
}

#[derive(Parser, Debug)]
struct WatchCli {
    #[arg(long)]
    state: PathBuf,
    #[arg(long)]
    lock: PathBuf,
    #[arg(long, default_value_t = IDLE_TTL_SECS)]
    idle_seconds: i64,
}

#[derive(Clone, Copy, Debug, Serialize)]
enum EnvName {
    Beta,
    Staging,
    StagingIgnis,
    Prod,
    Local,
}

impl EnvName {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "beta" => Ok(Self::Beta),
            "staging" => Ok(Self::Staging),
            "staging-ignis" => Ok(Self::StagingIgnis),
            "prod" | "production" => Ok(Self::Prod),
            "local" => Ok(Self::Local),
            _ => bail!("Unsupported env: {value}"),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Beta => "beta",
            Self::Staging => "staging",
            Self::StagingIgnis => "staging-ignis",
            Self::Prod => "prod",
            Self::Local => "local",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
enum ServiceName {
    Back,
    Preview,
    Plugins,
    Localhost,
}

impl ServiceName {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "back" => Ok(Self::Back),
            "preview" => Ok(Self::Preview),
            "plugins" => Ok(Self::Plugins),
            "localhost" => Ok(Self::Localhost),
            _ => bail!("Unsupported service: {value}"),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Back => "back",
            Self::Preview => "preview",
            Self::Plugins => "plugins",
            Self::Localhost => "localhost",
        }
    }
}

#[derive(Clone, Debug)]
struct RuntimeConfig {
    env: EnvName,
    service: ServiceName,
    sql_payload: String,
    op_account: String,
    local_port: u16,
    postgres_port: Option<u16>,
    ssm_port: u16,
    item_id: Option<String>,
    aws_profile_override: Option<String>,
    ssm_target_override: Option<String>,
    op_cache_ttl_seconds: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DbConfig {
    db_host: String,
    db_port: u16,
    db_name: String,
    db_user: String,
    db_pass: String,
    ssh_enabled: bool,
    ssh_key: Option<String>,
    aws_profile: Option<String>,
    bastion_instance_id: Option<String>,
    bastion_host: Option<String>,
    ssh_user: Option<String>,
    requires_tunnel: bool,
}

#[derive(Clone, Debug, Serialize)]
struct TunnelSpec {
    env: &'static str,
    service: &'static str,
    local_port: u16,
    remote_host: String,
    remote_port: u16,
    transport: &'static str,
    bastion_reference: Option<String>,
    aws_profile: Option<String>,
    ssh_user: Option<String>,
    ssh_key: Option<String>,
}

#[derive(Clone, Debug)]
struct TunnelLease {
    state_path: PathBuf,
    lock_path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TunnelState {
    fingerprint: String,
    pid: i32,
    watcher_pid: Option<i32>,
    local_port: u16,
    transport: String,
    remote_host: String,
    remote_port: u16,
    log_path: String,
    last_used_unix: i64,
}

#[derive(Serialize, Deserialize)]
struct CachedDbConfig {
    item_id: String,
    cached_at_unix: i64,
    db: DbConfig,
}

#[derive(Deserialize)]
struct OpItem {
    fields: Vec<OpField>,
}

#[derive(Deserialize)]
struct OpField {
    label: Option<String>,
    value: Option<String>,
}

fn main() {
    let result = if is_watch_mode() {
        run_watch_mode()
    } else {
        run_public_mode()
    };

    match result {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
    }
}

fn is_watch_mode() -> bool {
    matches!(std::env::args().nth(1).as_deref(), Some("__watch-tunnel"))
}

fn run_watch_mode() -> Result<i32> {
    let args: Vec<OsString> = std::env::args_os().collect();
    let cli = WatchCli::parse_from(std::iter::once(args[0].clone()).chain(args.into_iter().skip(2)));
    run_tunnel_watcher(cli.state, cli.lock, cli.idle_seconds)?;
    Ok(0)
}

fn run_public_mode() -> Result<i32> {
    let cli = PublicCli::parse();
    let runtime = build_runtime_config(cli)?;
    let db = match runtime.env {
        EnvName::Local => local_db_config(runtime.service),
        EnvName::Beta => {
            let tunnel_aws_profile = runtime
                .aws_profile_override
                .clone()
                .unwrap_or_else(|| "DataBeta".to_owned());
            let credentials_aws_profile = "weweb-beta";
            ensure_aws_profile_login(credentials_aws_profile)?;
            ensure_aws_profile_login(&tunnel_aws_profile)?;
            load_beta_db_config(runtime.service, credentials_aws_profile, &tunnel_aws_profile)?
        }
        _ => {
            let item_id = runtime
                .item_id
                .as_deref()
                .context("Missing 1Password item id")?;
            load_db_config_cached(item_id, &runtime.op_account, runtime.op_cache_ttl_seconds)?
        }
    };

    let lease = match runtime.env {
        EnvName::Prod if db.requires_tunnel => {
            ensure_aws_login(&runtime, &db)?;
            let spec = build_prod_bastion_ssh_tunnel_spec(&runtime, &db)?;
            Some(ensure_tunnel(&spec)?)
        }
        _ if should_use_aws_ssm_tunnel(&runtime, &db) => {
            ensure_aws_login(&runtime, &db)?;
            let spec = build_aws_ssm_tunnel_spec(&runtime, &db)?;
            Some(ensure_tunnel(&spec)?)
        }
        EnvName::Local => None,
        _ if db.ssh_enabled || db.requires_tunnel => {
            let spec = build_ssh_tunnel_spec(&runtime, &db)?;
            Some(ensure_tunnel(&spec)?)
        }
        _ => None,
    };

    let (stop_heartbeat, heartbeat_handle) = if let Some(lease) = lease.clone() {
        touch_tunnel_state(&lease)?;
        let (stop_tx, stop_rx) = mpsc::channel();
        let handle = spawn_tunnel_heartbeat(lease, stop_rx);
        (Some(stop_tx), Some(handle))
    } else {
        (None, None)
    };

    let status_code = match runtime.env {
        EnvName::Local => run_local_psql(&runtime.sql_payload, &db, runtime.postgres_port)?,
        EnvName::Prod if db.requires_tunnel => run_prod_psql(&runtime.sql_payload, &db, &runtime)?,
        _ => run_remote_psql(&runtime.sql_payload, &db, runtime.local_port)?,
    };

    if let Some(stop) = stop_heartbeat {
        let _ = stop.send(());
    }
    if let Some(handle) = heartbeat_handle {
        let _ = handle.join();
    }

    Ok(status_code)
}

fn build_runtime_config(cli: PublicCli) -> Result<RuntimeConfig> {
    if cli.sql.is_some() && cli.file.is_some() {
        bail!("Use only one of --sql or --file");
    }

    let env = EnvName::parse(&cli.env)?;
    let service = match cli.service.as_deref() {
        Some(value) => ServiceName::parse(value)?,
        None => default_service_for_env(env),
    };
    if cli.postgres_port.is_some() && !matches!(env, EnvName::Local) {
        bail!("--postgres-port is only supported with --env local");
    }
    let sql_payload = read_sql_payload(cli.sql, cli.file)?;
    let item_id = match env {
        EnvName::Beta | EnvName::Local => None,
        _ => Some(resolve_item_id(env, service)?),
    };

    Ok(RuntimeConfig {
        env,
        service,
        sql_payload,
        op_account: cli.op_account,
        local_port: cli.local_port,
        postgres_port: cli.postgres_port,
        ssm_port: cli.ssm_port,
        item_id,
        aws_profile_override: cli.aws_profile,
        ssm_target_override: cli.ssm_target,
        op_cache_ttl_seconds: cli.op_cache_ttl_seconds,
    })
}

fn default_service_for_env(env: EnvName) -> ServiceName {
    match env {
        EnvName::Local => ServiceName::Localhost,
        EnvName::Beta | EnvName::Staging | EnvName::StagingIgnis | EnvName::Prod => ServiceName::Back,
    }
}

fn resolve_item_id(env: EnvName, service: ServiceName) -> Result<String> {
    let item_id = match (env, service) {
        (EnvName::Staging, ServiceName::Back) => Some("k7i2nla3wxsvy7zmn5buqouan4"),
        (EnvName::Staging, ServiceName::Preview) => Some("n3ntvnicjwg3np2hyapqfnh57a"),
        (EnvName::Staging, ServiceName::Plugins) => Some("7ujhrwvjz6tee5ld6swvj7w4nm"),
        (EnvName::StagingIgnis, ServiceName::Back) => Some("yoyuvrpxyh66xgsgi5voawstpm"),
        (EnvName::StagingIgnis, ServiceName::Preview) => Some("yuzyrkgv6yovwa4yoz6y6gr76a"),
        (EnvName::StagingIgnis, ServiceName::Plugins) => Some("dn6zxuytzn4yqhrd2fvw23hjm4"),
        (EnvName::Prod, ServiceName::Back) => Some("2vdglcc4lhmoapd66puvisnx4e"),
        (EnvName::Prod, ServiceName::Preview) => Some("jjjkgtmkaovhxccpztm3o6cnqi"),
        (EnvName::Prod, ServiceName::Plugins) => Some("pfnt2drqepqtp2wgce3xw76dtu"),
        _ => None,
    };

    item_id
        .map(str::to_owned)
        .ok_or_else(|| anyhow::anyhow!("Missing 1Password item id for {}:{}", env.as_str(), service.as_str()))
}

fn local_db_config(service: ServiceName) -> DbConfig {
    let db_name = match service {
        ServiceName::Localhost | ServiceName::Back => "wwdb",
        ServiceName::Preview => "wwpreview",
        ServiceName::Plugins => "wwplugins",
    };

    DbConfig {
        db_host: "postgres".to_owned(),
        db_port: 5432,
        db_name: db_name.to_owned(),
        db_user: db_name.to_owned(),
        db_pass: "wwpassword".to_owned(),
        ssh_enabled: false,
        ssh_key: None,
        aws_profile: None,
        bastion_instance_id: None,
        bastion_host: None,
        ssh_user: None,
        requires_tunnel: false,
    }
}

fn load_beta_db_config(service: ServiceName, credentials_aws_profile: &str, tunnel_aws_profile: &str) -> Result<DbConfig> {
    let service_name = match service {
        ServiceName::Back => "weweb-back",
        ServiceName::Preview => "weweb-preview",
        ServiceName::Plugins => "weweb-plugins",
        ServiceName::Localhost => bail!("Service localhost is only supported with --env local"),
    };
    let parameter_prefix = format!("/beta/{service_name}");
    let parameter_names = [
        "rds_hostname",
        "rds_port",
        "rds_db_name",
        "rds_username",
        "rds_password",
    ]
    .map(|name| format!("{parameter_prefix}/{name}"));

    let output = Command::new("aws")
        .args([
            "ssm",
            "get-parameters",
            "--with-decryption",
            "--profile",
            credentials_aws_profile,
            "--output",
            "json",
            "--names",
        ])
        .args(&parameter_names)
        .output()
        .context("Failed to read beta database parameters from AWS SSM")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!("Failed to read beta database parameters from AWS SSM: {stderr}");
    }

    let payload: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("Failed to parse beta database parameters from AWS SSM")?;
    let parameters = payload["Parameters"]
        .as_array()
        .context("AWS SSM response is missing Parameters")?;
    let read_parameter = |name: &str| -> Result<String> {
        let full_name = format!("{parameter_prefix}/{name}");
        parameters
            .iter()
            .find(|parameter| parameter["Name"].as_str() == Some(&full_name))
            .and_then(|parameter| parameter["Value"].as_str())
            .map(str::to_owned)
            .with_context(|| format!("Missing beta database parameter {full_name}"))
    };

    Ok(DbConfig {
        db_host: read_parameter("rds_hostname")?,
        db_port: read_parameter("rds_port")?
            .parse()
            .context("Invalid beta database port")?,
        db_name: read_parameter("rds_db_name")?,
        db_user: read_parameter("rds_username")?,
        db_pass: read_parameter("rds_password")?,
        ssh_enabled: false,
        ssh_key: None,
        aws_profile: Some(tunnel_aws_profile.to_owned()),
        bastion_instance_id: None,
        bastion_host: None,
        ssh_user: None,
        requires_tunnel: true,
    })
}

fn read_sql_payload(sql: Option<String>, file: Option<PathBuf>) -> Result<String> {
    if let Some(sql) = sql {
        if sql.trim().is_empty() {
            bail!("Empty SQL input");
        }
        return Ok(sql);
    }

    if let Some(path) = file {
        let payload = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read SQL file {}", path.display()))?;
        if payload.trim().is_empty() {
            bail!("Empty SQL input");
        }
        return Ok(payload);
    }

    if io::stdin().is_terminal() {
        bail!("Provide --sql, --file, or stdin SQL");
    }

    let mut payload = String::new();
    io::stdin()
        .read_to_string(&mut payload)
        .context("Failed to read SQL from stdin")?;
    if payload.trim().is_empty() {
        bail!("Empty SQL input");
    }
    Ok(payload)
}

fn ensure_op_signin(account: &str) -> Result<()> {
    let signed_in = Command::new("op")
        .arg("whoami")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Failed to execute op whoami")?
        .success();

    if signed_in {
        return Ok(());
    }

    let status = Command::new("op")
        .args(["signin", "--account", account])
        .status()
        .context("Failed to execute op signin")?;

    if !status.success() {
        bail!("1Password sign-in failed");
    }

    Ok(())
}

fn load_db_config_cached(item_id: &str, account: &str, ttl_seconds: i64) -> Result<DbConfig> {
    if ttl_seconds <= 0 {
        ensure_op_signin(account)?;
        return load_db_config(item_id);
    }

    let cache_dir = ensure_db_config_cache_dir()?;
    let fingerprint = blake3::hash(item_id.as_bytes()).to_hex()[..24].to_owned();
    let cache_path = cache_dir.join(format!("{fingerprint}.json"));
    let lock_path = cache_dir.join(format!("{fingerprint}.lock"));
    let lock_file = open_lock_file(&lock_path)?;
    lock_file
        .lock_exclusive()
        .context("Failed to acquire DB config cache lock")?;

    if let Some(db) = read_db_config_cache(&cache_path, item_id, ttl_seconds)? {
        drop(lock_file);
        return Ok(db);
    }

    ensure_op_signin(account)?;
    let db = load_db_config(item_id)?;
    write_db_config_cache(&cache_path, item_id, &db)?;
    drop(lock_file);
    Ok(db)
}

fn ensure_db_config_cache_dir() -> Result<PathBuf> {
    let state_dir = Path::new(STATE_DIR);
    fs::create_dir_all(state_dir).context("Failed to create state directory")?;
    fs::set_permissions(state_dir, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("Failed to chmod {}", state_dir.display()))?;

    let cache_dir = state_dir.join("op-cache");
    fs::create_dir_all(&cache_dir).context("Failed to create DB config cache directory")?;
    fs::set_permissions(&cache_dir, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("Failed to chmod {}", cache_dir.display()))?;
    Ok(cache_dir)
}

fn read_db_config_cache(path: &Path, item_id: &str, ttl_seconds: i64) -> Result<Option<DbConfig>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read DB config cache {}", path.display()))?;
    let cache: CachedDbConfig = match serde_json::from_str(&content) {
        Ok(cache) => cache,
        Err(_) => {
            let _ = fs::remove_file(path);
            return Ok(None);
        }
    };

    if cache.item_id != item_id || now_unix() - cache.cached_at_unix > ttl_seconds {
        let _ = fs::remove_file(path);
        return Ok(None);
    }

    Ok(Some(cache.db))
}

fn write_db_config_cache(path: &Path, item_id: &str, db: &DbConfig) -> Result<()> {
    let cache = CachedDbConfig {
        item_id: item_id.to_owned(),
        cached_at_unix: now_unix(),
        db: db.clone(),
    };
    let serialized = serde_json::to_string(&cache).context("Failed to serialize DB config cache")?;
    let temp_path = path.with_extension(format!("json.{}.tmp", std::process::id()));
    fs::write(&temp_path, format!("{serialized}\n"))
        .with_context(|| format!("Failed to write DB config cache {}", temp_path.display()))?;
    fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("Failed to chmod {}", temp_path.display()))?;
    fs::rename(&temp_path, path)
        .with_context(|| format!("Failed to replace DB config cache {}", path.display()))?;
    Ok(())
}

fn load_db_config(item_id: &str) -> Result<DbConfig> {
    let output = Command::new("op")
        .args(["item", "get", item_id, "--vault", VAULT, "--format", "json"])
        .output()
        .with_context(|| format!("Failed to execute op item get {item_id}"))?;

    if !output.status.success() {
        bail!(
            "Failed to read 1Password item {}: {}",
            item_id,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let item: OpItem = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("Failed to parse 1Password item {}", item_id))?;

    let db_host = field_value(&item, "server").context("Missing DB host in 1Password item")?;
    let db_port = field_value(&item, "port")
        .unwrap_or_else(|| "5432".to_owned())
        .parse::<u16>()
        .context("Invalid DB port in 1Password item")?;
    let db_name = field_value(&item, "database").context("Missing DB name in 1Password item")?;
    let db_user = field_value(&item, "username").context("Missing DB user in 1Password item")?;
    let db_pass = field_value(&item, "password").context("Missing DB password in 1Password item")?;

    Ok(DbConfig {
        db_host,
        db_port,
        db_name,
        db_user,
        db_pass,
        ssh_enabled: parse_bool(field_value(&item, "ssh_enabled")),
        ssh_key: field_value(&item, "ssh_key"),
        aws_profile: field_value(&item, "aws_profile"),
        bastion_instance_id: field_value(&item, "bastion_instance_id"),
        bastion_host: field_value(&item, "bastion_public_ip"),
        ssh_user: field_value(&item, "ssh_user"),
        requires_tunnel: parse_bool(field_value(&item, "requires_tunnel")),
    })
}

fn field_value(item: &OpItem, label: &str) -> Option<String> {
    item.fields
        .iter()
        .find(|field| field.label.as_deref() == Some(label))
        .and_then(|field| field.value.clone())
        .filter(|value| !value.is_empty())
}

fn parse_bool(value: Option<String>) -> bool {
    matches!(value.as_deref(), Some("true" | "1" | "yes" | "on"))
}

fn ensure_aws_login(runtime: &RuntimeConfig, db: &DbConfig) -> Result<()> {
    let aws_profile = runtime
        .aws_profile_override
        .clone()
        .or_else(|| db.aws_profile.clone())
        .unwrap_or_else(|| "DataProd".to_owned());

    ensure_aws_profile_login(&aws_profile)
}

fn ensure_aws_profile_login(aws_profile: &str) -> Result<()> {
    let sts_ok = Command::new("aws")
        .args(["sts", "get-caller-identity", "--profile", &aws_profile])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Failed to execute aws sts get-caller-identity")?
        .success();

    if sts_ok {
        return Ok(());
    }

    let status = Command::new("aws")
        .args(["sso", "login", "--profile", aws_profile])
        .status()
        .context("Failed to execute aws sso login")?;

    if !status.success() {
        bail!("AWS SSO login failed for profile {}", aws_profile);
    }

    Ok(())
}

fn should_use_aws_ssm_tunnel(runtime: &RuntimeConfig, db: &DbConfig) -> bool {
    !matches!(runtime.env, EnvName::Local | EnvName::Prod)
        && db.requires_tunnel
        && (runtime.aws_profile_override.is_some() || db.aws_profile.is_some())
}

fn build_prod_bastion_ssh_tunnel_spec(runtime: &RuntimeConfig, db: &DbConfig) -> Result<TunnelSpec> {
    let aws_profile = runtime
        .aws_profile_override
        .clone()
        .or_else(|| db.aws_profile.clone())
        .unwrap_or_else(|| "DataProd".to_owned());
    ensure_session_manager_plugin_available()?;
    let ssm_target = resolve_ssm_target(runtime, db, &aws_profile)?;
    Ok(TunnelSpec {
        env: runtime.env.as_str(),
        service: "shared-bastion-ssh",
        local_port: runtime.ssm_port,
        remote_host: "127.0.0.1".to_owned(),
        remote_port: 22,
        transport: "ssm-ssh",
        bastion_reference: Some(ssm_target),
        aws_profile: Some(aws_profile),
        ssh_user: None,
        ssh_key: None,
    })
}

fn build_aws_ssm_tunnel_spec(runtime: &RuntimeConfig, db: &DbConfig) -> Result<TunnelSpec> {
    let aws_profile = runtime
        .aws_profile_override
        .clone()
        .or_else(|| db.aws_profile.clone())
        .unwrap_or_else(|| "DataProd".to_owned());
    ensure_session_manager_plugin_available()?;
    let ssm_target = resolve_ssm_target(runtime, db, &aws_profile)?;
    Ok(TunnelSpec {
        env: runtime.env.as_str(),
        service: runtime.service.as_str(),
        local_port: runtime.local_port,
        remote_host: db.db_host.clone(),
        remote_port: db.db_port,
        transport: "ssm",
        bastion_reference: Some(ssm_target),
        aws_profile: Some(aws_profile),
        ssh_user: None,
        ssh_key: None,
    })
}

fn ensure_session_manager_plugin_available() -> Result<()> {
    if Command::new("session-manager-plugin")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
    {
        return Ok(());
    }

    bail!(
        "Missing AWS Session Manager plugin. Install aws-session-manager-plugin before using AWS SSM tunnels"
    );
}

fn resolve_ssm_target(runtime: &RuntimeConfig, db: &DbConfig, aws_profile: &str) -> Result<String> {
    if let Some(target) = &runtime.ssm_target_override {
        return Ok(target.clone());
    }

    if let Some(target) = discover_bastion_instance_from_ec2(runtime.env, aws_profile)? {
        return Ok(target);
    }

    if let Some(target) = &db.bastion_instance_id {
        if ssm_target_is_online(aws_profile, target)? {
            return Ok(target.clone());
        }
    }

    if matches!(runtime.env, EnvName::Prod) && !DEFAULT_PROD_SSM_TARGET.is_empty() {
        return Ok(DEFAULT_PROD_SSM_TARGET.to_owned());
    }

    let output = Command::new("aws")
        .args([
            "ssm",
            "describe-instance-information",
            "--profile",
            aws_profile,
            "--query",
            "InstanceInformationList[?PingStatus==`Online`].InstanceId | [0]",
            "--output",
            "text",
        ])
        .output()
        .context("Failed to discover an online AWS SSM target")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!("Failed to discover an online AWS SSM target: {stderr}");
    }

    let target = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if target.is_empty() || target == "None" {
        if let Some(configured_target) = &db.bastion_instance_id {
            bail!(
                "Configured AWS SSM target {configured_target} is not online and no online managed instance was found for profile {aws_profile}"
            );
        }

        bail!("Missing AWS SSM target and no online managed instance was found for profile {aws_profile}");
    }

    Ok(target)
}

fn discover_bastion_instance_from_ec2(env: EnvName, aws_profile: &str) -> Result<Option<String>> {
    let tag_name = match env {
        EnvName::Beta => "Weweb Bastion Host on beta",
        EnvName::Staging | EnvName::StagingIgnis => "Weweb Bastion Host on staging",
        EnvName::Prod => "Weweb Bastion Host on prod",
        EnvName::Local => return Ok(None),
    };
    let tag_filter = format!("Name=tag:Name,Values={tag_name}");

    let output = Command::new("aws")
        .args([
            "ec2",
            "describe-instances",
            "--profile",
            aws_profile,
            "--filters",
            "Name=instance-state-name,Values=running",
            &tag_filter,
            "--query",
            "Reservations[].Instances[].InstanceId | [0]",
            "--output",
            "text",
        ])
        .output()
        .context("Failed to discover bastion instance from EC2")?;

    if !output.status.success() {
        return Ok(None);
    }

    let target = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if target.is_empty() || target == "None" {
        return Ok(None);
    }

    Ok(Some(target))
}

fn ssm_target_is_online(aws_profile: &str, target: &str) -> Result<bool> {
    let filter = format!("Key=InstanceIds,Values={target}");
    let output = Command::new("aws")
        .args([
            "ssm",
            "describe-instance-information",
            "--profile",
            aws_profile,
            "--filters",
            &filter,
            "--query",
            "InstanceInformationList[0].PingStatus",
            "--output",
            "text",
        ])
        .output()
        .context("Failed to check configured AWS SSM target status")?;

    if !output.status.success() {
        return Ok(false);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim() == "Online")
}

fn build_ssh_tunnel_spec(runtime: &RuntimeConfig, db: &DbConfig) -> Result<TunnelSpec> {
    let ssh_key = db
        .ssh_key
        .clone()
        .context("ssh_enabled=true but ssh_key is missing in 1Password item")?;
    let bastion_host = db
        .bastion_host
        .clone()
        .context("Missing bastion host in 1Password item")?;
    let ssh_user = db
        .ssh_user
        .clone()
        .unwrap_or_else(|| "ec2-user".to_owned());

    Ok(TunnelSpec {
        env: runtime.env.as_str(),
        service: runtime.service.as_str(),
        local_port: runtime.local_port,
        remote_host: db.db_host.clone(),
        remote_port: db.db_port,
        transport: "ssh",
        bastion_reference: Some(bastion_host),
        aws_profile: None,
        ssh_user: Some(ssh_user),
        ssh_key: Some(ssh_key),
    })
}

fn ensure_tunnel(spec: &TunnelSpec) -> Result<TunnelLease> {
    fs::create_dir_all(STATE_DIR).context("Failed to create tunnel state directory")?;

    let fingerprint = tunnel_fingerprint(spec)?;
    let state_path = Path::new(STATE_DIR).join(format!("{fingerprint}.json"));
    let lock_path = Path::new(STATE_DIR).join(format!("{fingerprint}.lock"));
    let lock_file = open_lock_file(&lock_path)?;
    lock_file
        .lock_exclusive()
        .context("Failed to acquire tunnel state lock")?;

    if let Some(existing) = read_tunnel_state(&state_path)? {
        if existing.fingerprint == fingerprint && process_exists(existing.pid) && port_is_ready(existing.local_port) {
            let watcher_pid = match existing.watcher_pid {
                Some(pid) if process_exists(pid) => Some(pid),
                _ => Some(spawn_tunnel_watcher(&state_path, &lock_path)?),
            };
            let updated = TunnelState {
                watcher_pid,
                last_used_unix: now_unix(),
                ..existing
            };
            write_tunnel_state(&state_path, &updated)?;
            drop(lock_file);
            return Ok(TunnelLease { state_path, lock_path });
        }

        stop_process_group(existing.pid);
        if let Some(pid) = existing.watcher_pid {
            stop_process_group(pid);
        }
        let _ = fs::remove_file(&state_path);
    }

    ensure_tunnel_ports_available(spec)?;

    let log_path = Path::new(STATE_DIR).join(format!("{fingerprint}.tunnel.log"));
    let pid = spawn_tunnel(spec, &log_path)?;
    wait_for_tunnel_ready(spec.local_port, pid, &log_path)?;

    let mut new_state = TunnelState {
        fingerprint,
        pid,
        watcher_pid: None,
        local_port: spec.local_port,
        transport: spec.transport.to_owned(),
        remote_host: spec.remote_host.clone(),
        remote_port: spec.remote_port,
        log_path: log_path.display().to_string(),
        last_used_unix: now_unix(),
    };
    write_tunnel_state(&state_path, &new_state)?;

    let watcher_pid = spawn_tunnel_watcher(&state_path, &lock_path)?;
    new_state.watcher_pid = Some(watcher_pid);
    write_tunnel_state(&state_path, &new_state)?;

    drop(lock_file);

    Ok(TunnelLease { state_path, lock_path })
}

fn open_lock_file(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)
        .with_context(|| format!("Failed to open lock file {}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("Failed to chmod {}", path.display()))?;
    Ok(file)
}

fn tunnel_fingerprint(spec: &TunnelSpec) -> Result<String> {
    let bytes = serde_json::to_vec(spec).context("Failed to serialize tunnel fingerprint")?;
    Ok(blake3::hash(&bytes).to_hex()[..24].to_owned())
}

fn read_tunnel_state(path: &Path) -> Result<Option<TunnelState>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read tunnel state {}", path.display()))?;

    if content.trim().is_empty() {
        return Ok(None);
    }

    let state = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse tunnel state {}", path.display()))?;
    Ok(Some(state))
}

fn write_tunnel_state(path: &Path, state: &TunnelState) -> Result<()> {
    let serialized = serde_json::to_string_pretty(state).context("Failed to serialize tunnel state")?;
    fs::write(path, format!("{serialized}\n"))
        .with_context(|| format!("Failed to write tunnel state {}", path.display()))
}

fn spawn_tunnel(spec: &TunnelSpec, log_path: &Path) -> Result<i32> {
    match spec.transport {
        "ssm" => spawn_detached_command(build_ssm_tunnel_command(spec)?, log_path),
        "ssm-ssh" => spawn_detached_command(build_ssm_ssh_tunnel_command(spec)?, log_path),
        "ssh" => spawn_detached_command(build_ssh_tunnel_command(spec)?, log_path),
        _ => bail!("Unsupported tunnel transport {}", spec.transport),
    }
}

fn build_ssm_ssh_tunnel_command(spec: &TunnelSpec) -> Result<Command> {
    let ssm_target = spec
        .bastion_reference
        .clone()
        .context("Missing SSM target for prod bastion SSH tunnel")?;
    let aws_profile = spec
        .aws_profile
        .clone()
        .context("Missing AWS profile for prod bastion SSH tunnel")?;
    let mut command = Command::new("bash");
    command.arg("-lc");
    command.arg(format!(
        "set -euo pipefail\n\
	ssm_pid=''\n\
	cleanup() {{\n\
	  if [[ -n \"$ssm_pid\" ]]; then kill \"$ssm_pid\" 2>/dev/null || true; wait \"$ssm_pid\" 2>/dev/null || true; fi\n\
	}}\n\
	trap cleanup EXIT INT TERM\n\
	aws ssm start-session --target {ssm_target} --document-name AWS-StartPortForwardingSession --parameters {parameters} --profile {aws_profile} &\n\
	ssm_pid=$!\n\
	for _ in $(seq 1 60); do\n\
	  if (echo >/dev/tcp/127.0.0.1/{local_port}) >/dev/null 2>&1; then\n\
	    break\n\
	  fi\n\
	  if ! kill -0 \"$ssm_pid\" 2>/dev/null; then\n\
	    wait \"$ssm_pid\"\n\
	    exit $?\n\
	  fi\n\
	  sleep 0.25\n\
	done\n\
	if ! (echo >/dev/tcp/127.0.0.1/{local_port}) >/dev/null 2>&1; then\n\
	  exit 1\n\
	fi\n\
	wait \"$ssm_pid\"\n\
	exit $?",
        ssm_target = shell_escape(&ssm_target),
        parameters = shell_escape(&format!(
            "{{\"portNumber\":[\"{}\"],\"localPortNumber\":[\"{}\"]}}",
            spec.remote_port, spec.local_port
        )),
        aws_profile = shell_escape(&aws_profile),
        local_port = spec.local_port,
    ));
    Ok(command)
}

fn build_ssm_tunnel_command(spec: &TunnelSpec) -> Result<Command> {
    let ssm_target = spec
        .bastion_reference
        .clone()
        .context("Missing SSM target for AWS tunnel")?;
    let aws_profile = spec
        .aws_profile
        .clone()
        .context("Missing AWS profile for AWS tunnel")?;
    let mut command = Command::new("bash");
    command.arg("-lc");
    command.arg(format!(
        "set -euo pipefail\n\
	ssm_pid=''\n\
	cleanup() {{\n\
	  if [[ -n \"$ssm_pid\" ]]; then kill \"$ssm_pid\" 2>/dev/null || true; wait \"$ssm_pid\" 2>/dev/null || true; fi\n\
	}}\n\
	trap cleanup EXIT INT TERM\n\
	aws ssm start-session --target {ssm_target} --document-name AWS-StartPortForwardingSessionToRemoteHost --parameters {parameters} --profile {aws_profile} &\n\
	ssm_pid=$!\n\
	for _ in $(seq 1 60); do\n\
	  if (echo >/dev/tcp/127.0.0.1/{local_port}) >/dev/null 2>&1; then\n\
	    break\n\
	  fi\n\
	  if ! kill -0 \"$ssm_pid\" 2>/dev/null; then\n\
	    wait \"$ssm_pid\"\n\
	    exit $?\n\
	  fi\n\
	  sleep 0.25\n\
	done\n\
	if ! (echo >/dev/tcp/127.0.0.1/{local_port}) >/dev/null 2>&1; then\n\
	  exit 1\n\
	fi\n\
	wait \"$ssm_pid\"\n\
	exit $?",
        ssm_target = shell_escape(&ssm_target),
        parameters = shell_escape(&format!(
            "{{\"host\":[\"{}\"],\"portNumber\":[\"{}\"],\"localPortNumber\":[\"{}\"]}}",
            spec.remote_host, spec.remote_port, spec.local_port
        )),
        aws_profile = shell_escape(&aws_profile),
        local_port = spec.local_port,
    ));
    Ok(command)
}

fn build_ssh_tunnel_command(spec: &TunnelSpec) -> Result<Command> {
    let bastion_host = spec
        .bastion_reference
        .clone()
        .context("Missing bastion host for SSH tunnel")?;
    let ssh_user = spec
        .ssh_user
        .clone()
        .unwrap_or_else(|| "ec2-user".to_owned());
    let ssh_key = spec
        .ssh_key
        .clone()
        .context("Missing SSH key for SSH tunnel")?;

    let key_path = write_ssh_key_tempfile(&ssh_key)?;
    let mut command = Command::new("ssh");
    command.args([
        "-o",
        "BatchMode=yes",
        "-o",
        "IgnoreUnknown=WarnWeakCrypto",
        "-o",
        "WarnWeakCrypto=no-pq-kex",
        "-o",
        "ExitOnForwardFailure=yes",
        "-o",
        "StrictHostKeyChecking=no",
        "-o",
        "UserKnownHostsFile=/tmp/weweb_known_hosts",
        "-i",
        key_path.to_string_lossy().as_ref(),
        "-N",
        "-L",
        &format!("{}:{}:{}", spec.local_port, spec.remote_host, spec.remote_port),
        &format!("{ssh_user}@{bastion_host}"),
    ]);
    Ok(command)
}

fn write_ssh_key_tempfile(ssh_key: &str) -> Result<PathBuf> {
    for _ in 0..32 {
        let key_path = Path::new(STATE_DIR).join(format!(
            "ssh-key-{}-{}-{}",
            std::process::id(),
            now_unix_nanos(),
            TEMPFILE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&key_path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| format!("Failed to create SSH key {}", key_path.display()));
            }
        };
        file.write_all(format!("{ssh_key}\n").as_bytes())
            .with_context(|| format!("Failed to write SSH key {}", key_path.display()))?;
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Failed to chmod SSH key {}", key_path.display()))?;
        return Ok(key_path);
    }

    bail!("Failed to allocate a unique SSH key tempfile")
}

fn shell_escape(value: &str) -> String {
    shell_single_quote(value)
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn spawn_detached_command(mut command: Command, log_path: &Path) -> Result<i32> {
    let stdout_log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .with_context(|| format!("Failed to open tunnel log {}", log_path.display()))?;
    let stderr_log = stdout_log
        .try_clone()
        .with_context(|| format!("Failed to clone tunnel log {}", log_path.display()))?;

    command.stdin(Stdio::null());
    command.stdout(Stdio::from(stdout_log));
    command.stderr(Stdio::from(stderr_log));

    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let child = command.spawn().context("Failed to spawn detached command")?;
    Ok(child.id() as i32)
}

fn wait_for_tunnel_ready(local_port: u16, pid: i32, log_path: &Path) -> Result<()> {
    let start = now_unix();
    loop {
        if port_is_ready(local_port) {
            return Ok(());
        }
        if !process_exists(pid) {
            bail!(
                "Tunnel process exited early. {}",
                log_excerpt(log_path).unwrap_or_default()
            );
        }
        if now_unix() - start >= TUNNEL_READY_TIMEOUT_SECS as i64 {
            stop_process_group(pid);
            bail!(
                "Tunnel did not become ready. {}",
                log_excerpt(log_path).unwrap_or_default()
            );
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn log_excerpt(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lines: Vec<&str> = trimmed.lines().rev().take(10).collect();
    Some(format!("Log tail:\n{}", lines.into_iter().rev().collect::<Vec<_>>().join("\n")))
}

fn port_is_ready(port: u16) -> bool {
    ("127.0.0.1", port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
        .and_then(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(250)).ok())
        .is_some()
}

fn ensure_tunnel_ports_available(spec: &TunnelSpec) -> Result<()> {
    if port_is_ready(spec.local_port) {
        bail!(
            "Port {} is already in use without a matching managed tunnel state. Pick another --local-port or clean up the existing listener.",
            spec.local_port
        );
    }

    Ok(())
}

fn process_exists(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }
    unsafe {
        let result = libc::kill(pid, 0);
        if result == 0 {
            return true;
        }
        matches!(io::Error::last_os_error().raw_os_error(), Some(libc::EPERM))
    }
}

fn stop_process_group(pid: i32) {
    if pid <= 0 {
        return;
    }

    unsafe {
        libc::kill(-pid, libc::SIGTERM);
    }

    for _ in 0..10 {
        if !process_exists(pid) {
            return;
        }
        thread::sleep(Duration::from_millis(200));
    }

    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
}

fn spawn_tunnel_watcher(state_path: &Path, lock_path: &Path) -> Result<i32> {
    let script_path = current_script_path()?;
    let log_path = state_path.with_extension("watch.log");
    let mut command = Command::new(script_path);
    command.arg("__watch-tunnel");
    command.arg("--state");
    command.arg(state_path);
    command.arg("--lock");
    command.arg(lock_path);
    command.arg("--idle-seconds");
    command.arg(IDLE_TTL_SECS.to_string());
    spawn_detached_command(command, &log_path)
}

fn current_script_path() -> Result<PathBuf> {
    let wrapper = std::env::var_os("WEWEB_PSQL_SELF")
        .or_else(|| std::env::args_os().next())
        .context("Missing current script path")?;
    fs::canonicalize(wrapper).context("Failed to resolve current script path")
}

fn touch_tunnel_state(lease: &TunnelLease) -> Result<()> {
    let lock_file = open_lock_file(&lease.lock_path)?;
    lock_file
        .lock_exclusive()
        .context("Failed to acquire tunnel state lock")?;
    if let Some(mut state) = read_tunnel_state(&lease.state_path)? {
        state.last_used_unix = now_unix();
        write_tunnel_state(&lease.state_path, &state)?;
    }
    drop(lock_file);
    Ok(())
}

fn spawn_tunnel_heartbeat(lease: TunnelLease, stop: mpsc::Receiver<()>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            match stop.recv_timeout(Duration::from_secs(TOUCH_INTERVAL_SECS)) {
                Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            let _ = touch_tunnel_state(&lease);
        }
    })
}

fn run_tunnel_watcher(state_path: PathBuf, lock_path: PathBuf, idle_seconds: i64) -> Result<()> {
    loop {
        thread::sleep(Duration::from_secs(5));
        let lock_file = open_lock_file(&lock_path)?;
        lock_file
            .lock_exclusive()
            .context("Failed to acquire tunnel watcher lock")?;

        let Some(state) = read_tunnel_state(&state_path)? else {
            drop(lock_file);
            return Ok(());
        };

        if !process_exists(state.pid) {
            let _ = fs::remove_file(&state_path);
            drop(lock_file);
            return Ok(());
        }

        if now_unix() - state.last_used_unix > idle_seconds {
            stop_process_group(state.pid);
            let _ = fs::remove_file(&state_path);
            drop(lock_file);
            return Ok(());
        }

        drop(lock_file);
    }
}

fn run_remote_psql(sql: &str, db: &DbConfig, local_port: u16) -> Result<i32> {
    let connection_string = format!(
        "host=127.0.0.1 port={} dbname={} user={} sslmode=require connect_timeout=10",
        local_port, db.db_name, db.db_user
    );

    let mut command = Command::new("psql");
    command.arg(connection_string);
    command.args(["-v", "ON_ERROR_STOP=1", "-P", "pager=off"]);
    command.env("PGPASSWORD", &db.db_pass);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    run_sql_command(command, sql)
}

fn run_prod_psql(sql: &str, db: &DbConfig, runtime: &RuntimeConfig) -> Result<i32> {
    if port_is_ready(runtime.local_port) {
        bail!(
            "Port {} is already in use. Pick another --local-port or clean up the existing listener.",
            runtime.local_port
        );
    }

    let ssh_user = db
        .ssh_user
        .clone()
        .unwrap_or_else(|| "ec2-user".to_owned());
    let ssh_key = db
        .ssh_key
        .clone()
        .context("Missing SSH key for prod bastion hop")?;
    let key_path = write_ssh_key_tempfile(&ssh_key)?;
    let connection_string = format!(
        "host=127.0.0.1 port={} dbname={} user={} sslmode=require connect_timeout=10",
        runtime.local_port, db.db_name, db.db_user
    );

    let mut command = Command::new("bash");
    command.arg("-lc");
    command.arg(format!(
        "set -euo pipefail\n\
	ssh_pid=''\n\
	cleanup() {{\n\
	  if [[ -n \"$ssh_pid\" ]]; then kill \"$ssh_pid\" 2>/dev/null || true; wait \"$ssh_pid\" 2>/dev/null || true; fi\n\
	  rm -f {key_path}\n\
	}}\n\
	trap cleanup EXIT INT TERM\n\
		ssh -o BatchMode=yes -o IgnoreUnknown=WarnWeakCrypto -o WarnWeakCrypto=no-pq-kex -o ExitOnForwardFailure=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/tmp/weweb_known_hosts -i {key_path} -p {ssm_port} -N -L {local_port}:{remote_host}:{remote_port} {ssh_user}@127.0.0.1 &\n\
	ssh_pid=$!\n\
	for _ in $(seq 1 60); do\n\
	  if (echo >/dev/tcp/127.0.0.1/{local_port}) >/dev/null 2>&1; then\n\
	    break\n\
	  fi\n\
	  if ! kill -0 \"$ssh_pid\" 2>/dev/null; then\n\
	    wait \"$ssh_pid\"\n\
	    exit $?\n\
	  fi\n\
	  sleep 0.25\n\
	done\n\
	if ! (echo >/dev/tcp/127.0.0.1/{local_port}) >/dev/null 2>&1; then\n\
	  exit 1\n\
	fi\n\
	psql {connection_string} -v ON_ERROR_STOP=1 -P pager=off",
        key_path = shell_escape(key_path.to_string_lossy().as_ref()),
        ssm_port = runtime.ssm_port,
        local_port = runtime.local_port,
        remote_host = shell_escape(&db.db_host),
        remote_port = db.db_port,
        ssh_user = shell_escape(&ssh_user),
        connection_string = shell_escape(&connection_string),
    ));
    command.env("PGPASSWORD", &db.db_pass);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    run_sql_command(command, sql)
}

fn run_local_psql(sql: &str, db: &DbConfig, postgres_port: Option<u16>) -> Result<i32> {
    if let Some(port) = postgres_port {
        return run_local_tcp_psql(sql, db, port);
    }

    let compose_dir = resolve_weweb_docker_dir()?;
    let mut command = Command::new("docker");
    command.current_dir(compose_dir);
    command.args([
        "compose",
        "exec",
        "-T",
        "postgres",
        "psql",
        "-U",
        &db.db_user,
        "-d",
        &db.db_name,
        "-v",
        "ON_ERROR_STOP=1",
        "-P",
        "pager=off",
    ]);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    run_sql_command(command, sql)
}

fn run_local_tcp_psql(sql: &str, db: &DbConfig, postgres_port: u16) -> Result<i32> {
    let connection_string = format!(
        "host=127.0.0.1 port={} dbname={} user={} connect_timeout=10",
        postgres_port, db.db_name, db.db_user
    );

    let mut command = Command::new("psql");
    command.arg(connection_string);
    command.args(["-v", "ON_ERROR_STOP=1", "-P", "pager=off"]);
    command.env("PGPASSWORD", &db.db_pass);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    run_sql_command(command, sql)
}

fn run_sql_command(mut command: Command, sql: &str) -> Result<i32> {
    let mut child = command.spawn().context("Failed to spawn SQL command")?;
    {
        let mut stdin = child.stdin.take().context("Failed to open SQL stdin")?;
        stdin
            .write_all(sql.as_bytes())
            .context("Failed to write SQL payload")?;
    }
    let status = child.wait().context("Failed to wait for SQL command")?;
    Ok(status.code().unwrap_or(1))
}

fn resolve_weweb_docker_dir() -> Result<PathBuf> {
    if let Ok(value) = std::env::var("WEWEB_DOCKER_DIR") {
        let path = PathBuf::from(value);
        if path.exists() {
            return Ok(path);
        }
    }

    let default = PathBuf::from(DEFAULT_LOCAL_DOCKER_DIR);
    if default.exists() {
        return Ok(default);
    }

    bail!("Unable to resolve weweb-docker directory")
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}

fn now_unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_nanos()
}
