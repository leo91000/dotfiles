#!/usr/bin/env -S cargo +nightly -q -Zscript
---cargo
[package]
edition = "2024"

[dependencies]
aes = "0.8"
aes-gcm = "0.10"
base64 = "0.22"
cbc = { version = "0.1", features = ["alloc", "block-padding"] }
chrono = "0.4"
getrandom = "0.3"
hex = "0.4"
hkdf = "0.12"
pbkdf2 = "0.12"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha1 = "0.10"
sha2 = "0.10"
tempfile = "3"
url = "2"
---

use aes::Aes128;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use cbc::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
use chrono::Utc;
use hkdf::Hkdf;
use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use url::Url;

const CHROME_EPOCH_OFFSET_SECONDS: i64 = 11_644_473_600;
const ENVELOPE_FORMAT: &str = "browser-cookie-capsule";
const CAPSULE_FORMAT: &str = "browser-session-capsule";
const VERSION: u8 = 1;
const CIPHER: &str = "AES-256-GCM";
const KDF_NAME: &str = "HKDF-SHA256";
const KDF_INFO: &str = "browser-cookie-capsule/v1";
const DEFAULT_KEY_REFERENCE: &str = "op://WeWeb/lejs3pw5nntahgdm3wdfrzofqe/password";

type Aes128CbcDec = cbc::Decryptor<Aes128>;

#[derive(Debug)]
struct Cli {
    command: String,
    target: Url,
    related_origins: Vec<Url>,
    capsule: Option<PathBuf>,
    output: Option<PathBuf>,
    profile: String,
    user_data_dir: PathBuf,
    key_reference: String,
    timeout_seconds: u64,
}

#[derive(Debug, Deserialize)]
struct ChromeRow {
    host_key: String,
    top_frame_site_key: String,
    name: String,
    value: String,
    path: String,
    expires_utc: String,
    is_secure: i64,
    is_httponly: i64,
    has_expires: i64,
    samesite: i64,
    encrypted_hex: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Cookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    expires: i64,
    http_only: bool,
    secure: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    same_site: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    partition_key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Capsule {
    format: String,
    version: u8,
    created_at: String,
    #[serde(default)]
    target_origins: Vec<String>,
    source: CapsuleSource,
    storage_state: StorageState,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CapsuleSource {
    browser: String,
    profile: String,
    cookie_database_schema_version: i64,
}

#[derive(Debug, Deserialize, Serialize)]
struct StorageState {
    cookies: Vec<Cookie>,
    origins: Vec<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Envelope {
    format: String,
    version: u8,
    cipher: String,
    kdf: EnvelopeKdf,
    nonce: String,
    tag: String,
    ciphertext: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct EnvelopeKdf {
    name: String,
    salt: String,
    info: String,
}

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = parse_cli()?;
    match cli.command.as_str() {
        "export" => export_capsule(&cli),
        "inspect" => inspect_capsule(&cli),
        "serve" => serve_capsule(&cli),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "Usage:\n  session-capsule.rs export --target URL [--related-origin URL] [--output PATH] [--profile NAME]\n  session-capsule.rs inspect --capsule PATH --target URL [--related-origin URL]\n  session-capsule.rs serve --capsule PATH --target URL [--related-origin URL] [--timeout-seconds N]".to_owned()
}

fn parse_cli() -> Result<Cli, String> {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().ok_or_else(usage)?;
    let mut target = None;
    let mut related_origins = Vec::new();
    let mut capsule = None;
    let mut output = None;
    let mut profile = "Default".to_owned();
    let mut user_data_dir = default_chrome_user_data_dir()?;
    let mut key_reference = DEFAULT_KEY_REFERENCE.to_owned();
    let mut timeout_seconds = 45;

    while let Some(argument) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("Missing value for {argument}"))?;
        match argument.as_str() {
            "--target" => target = Some(parse_http_url(&value)?),
            "--related-origin" => related_origins.push(parse_http_url(&value)?),
            "--capsule" => capsule = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            "--profile" => profile = value,
            "--user-data-dir" => user_data_dir = PathBuf::from(value),
            "--key-ref" => key_reference = value,
            "--timeout-seconds" => {
                timeout_seconds = value
                    .parse::<u64>()
                    .map_err(|_| "--timeout-seconds must be a positive integer".to_owned())?;
                if timeout_seconds == 0 {
                    return Err("--timeout-seconds must be greater than zero".to_owned());
                }
            }
            _ => return Err(format!("Unknown argument: {argument}\n{}", usage())),
        }
    }

    let target = target.ok_or_else(|| format!("--target is required\n{}", usage()))?;
    if matches!(command.as_str(), "inspect" | "serve") && capsule.is_none() {
        return Err(format!("--capsule is required for {command}"));
    }

    Ok(Cli {
        command,
        target,
        related_origins,
        capsule,
        output,
        profile,
        user_data_dir,
        key_reference,
        timeout_seconds,
    })
}

fn parse_http_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_| format!("Invalid URL: {value}"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(format!("URL must be an absolute HTTP(S) URL: {value}"));
    }
    Ok(url)
}

fn default_chrome_user_data_dir() -> Result<PathBuf, String> {
    let user_home = env::var_os("HOME").ok_or_else(|| "HOME is unavailable".to_owned())?;
    Ok(PathBuf::from(user_home).join(".config/google-chrome"))
}

fn all_origins(cli: &Cli) -> Vec<Url> {
    let mut origins = vec![cli.target.clone()];
    for origin in &cli.related_origins {
        if !origins
            .iter()
            .any(|candidate| candidate.origin() == origin.origin())
        {
            origins.push(origin.clone());
        }
    }
    origins
}

fn export_capsule(cli: &Cli) -> Result<(), String> {
    let profile_dir = cli.user_data_dir.join(&cli.profile);
    let database_path = profile_dir.join("Cookies");
    if !database_path.is_file() {
        return Err(format!(
            "Chrome cookie database was not found for profile {}",
            cli.profile
        ));
    }

    let snapshot = snapshot_database(&database_path)?;
    let snapshot_database = snapshot.path().join("Cookies");
    let schema_version = query_schema_version(&snapshot_database)?;
    let chrome_secret = command_secret("secret-tool", &["lookup", "application", "chrome"])
        .map_err(|_| "Chrome keyring secret is unavailable".to_owned())?;
    let origins = all_origins(cli);
    let rows = query_cookies(&snapshot_database)?;
    let mut cookies = Vec::new();

    for row in rows {
        if !origins
            .iter()
            .any(|origin| domain_matches(&row.host_key, origin.host_str().unwrap_or_default()))
        {
            continue;
        }
        let value = decrypt_chrome_cookie(&row, schema_version, &chrome_secret)?;
        cookies.push(Cookie {
            name: row.name,
            value,
            domain: row.host_key,
            path: row.path,
            expires: chrome_timestamp_to_unix_seconds(&row.expires_utc, row.has_expires != 0)?,
            http_only: row.is_httponly != 0,
            secure: row.is_secure != 0,
            same_site: map_same_site(row.samesite)?,
            partition_key: (!row.top_frame_site_key.is_empty()).then_some(row.top_frame_site_key),
        });
    }

    let secret = read_encryption_key(&cli.key_reference)?;
    let capsule = Capsule {
        format: CAPSULE_FORMAT.to_owned(),
        version: VERSION,
        created_at: Utc::now().to_rfc3339(),
        target_origins: origins
            .iter()
            .map(|origin| origin.origin().ascii_serialization())
            .collect(),
        source: CapsuleSource {
            browser: "google-chrome".to_owned(),
            profile: cli.profile.clone(),
            cookie_database_schema_version: schema_version,
        },
        storage_state: StorageState {
            cookies,
            origins: Vec::new(),
        },
    };
    let envelope = encrypt_capsule(&capsule, secret.as_bytes())?;
    let round_trip = decrypt_capsule(&envelope, secret.as_bytes())?;
    if round_trip.storage_state.cookies.len() != capsule.storage_state.cookies.len() {
        return Err("Encrypted capsule round-trip validation failed".to_owned());
    }

    let output_path = match &cli.output {
        Some(path) => path.clone(),
        None => default_output_path(&cli.target)?,
    };
    write_private_new_file(
        &output_path,
        &serde_json::to_vec_pretty(&envelope).map_err(display_error)?,
    )?;

    println!(
        "{}",
        json!({
            "event": "exported",
            "outputPath": output_path,
            "targetOrigins": capsule.target_origins,
            "cookieCount": capsule.storage_state.cookies.len(),
            "partitionedCookieCount": capsule.storage_state.cookies.iter().filter(|cookie| cookie.partition_key.is_some()).count(),
            "schemaVersion": schema_version,
            "roundTripValidated": true,
        })
    );
    Ok(())
}

fn inspect_capsule(cli: &Cli) -> Result<(), String> {
    let capsule = load_capsule(cli)?;
    let cookies = select_cookies(&capsule.storage_state.cookies, &all_origins(cli));
    println!(
        "{}",
        metadata_json("inspected", &all_origins(cli), &cookies, &capsule)
    );
    Ok(())
}

fn serve_capsule(cli: &Cli) -> Result<(), String> {
    let capsule = load_capsule(cli)?;
    let origins = all_origins(cli);
    let cookies = select_cookies(&capsule.storage_state.cookies, &origins);
    if cookies.is_empty() {
        return Err(
            "The capsule contains no unexpired cookies matching the requested origins".to_owned(),
        );
    }

    let mut request_token = [0_u8; 32];
    getrandom::fill(&mut request_token).map_err(display_error)?;
    let request_token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(request_token);
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(display_error)?;
    listener.set_nonblocking(true).map_err(display_error)?;
    let address = listener.local_addr().map_err(display_error)?;
    let one_shot_url = format!("http://127.0.0.1:{}/{}", address.port(), request_token);
    let ready = metadata_json("ready", &origins, &cookies, &capsule);
    let mut ready = ready.as_object().cloned().unwrap_or_default();
    ready.insert("url".to_owned(), Value::String(one_shot_url));
    println!("{}", Value::Object(ready));
    std::io::stdout().flush().map_err(display_error)?;

    let deadline = Instant::now() + Duration::from_secs(cli.timeout_seconds);
    while Instant::now() < deadline {
        match listener.accept() {
            Ok((mut stream, _)) => {
                if serve_request(&mut stream, &request_token, &cookies)? {
                    println!("{}", metadata_json("served", &origins, &cookies, &capsule));
                    return Ok(());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(display_error(error)),
        }
    }
    Err("Bridge expired before cookies were requested".to_owned())
}

fn load_capsule(cli: &Cli) -> Result<Capsule, String> {
    let capsule_path = cli.capsule.as_ref().expect("validated by parse_cli");
    let envelope: Envelope =
        serde_json::from_slice(&fs::read(capsule_path).map_err(display_error)?)
            .map_err(|_| "Capsule envelope is not valid JSON".to_owned())?;
    let secret = read_encryption_key(&cli.key_reference)?;
    decrypt_capsule(&envelope, secret.as_bytes())
}

fn snapshot_database(database_path: &Path) -> Result<TempDir, String> {
    let snapshot = tempfile::tempdir().map_err(display_error)?;
    let target = snapshot.path().join("Cookies");
    let quoted_target = target.display().to_string().replace('\'', "''");
    let backup_command = format!(".backup '{quoted_target}'");
    let output = Command::new("sqlite3")
        .args([
            "-readonly",
            &database_path.display().to_string(),
            &backup_command,
        ])
        .output()
        .map_err(|_| "sqlite3 is unavailable".to_owned())?;
    if !output.status.success() {
        return Err("Unable to create a consistent Chrome cookie database snapshot".to_owned());
    }
    Ok(snapshot)
}

fn sqlite_json(database_path: &Path, query: &str) -> Result<String, String> {
    let database_uri = format!("file:{}?immutable=1", database_path.display());
    let output = Command::new("sqlite3")
        .args(["-readonly", &database_uri, "-json", query])
        .output()
        .map_err(|_| "sqlite3 is unavailable".to_owned())?;
    if !output.status.success() {
        return Err("Unable to query the Chrome cookie database snapshot".to_owned());
    }
    String::from_utf8(output.stdout).map_err(|_| "sqlite3 returned invalid UTF-8".to_owned())
}

fn query_schema_version(database_path: &Path) -> Result<i64, String> {
    let output = sqlite_json(
        database_path,
        "SELECT CAST(value AS INTEGER) AS version FROM meta WHERE key='version';",
    )?;
    let rows: Vec<Value> = serde_json::from_str(&output).map_err(display_error)?;
    rows.first()
        .and_then(|row| row.get("version"))
        .and_then(Value::as_i64)
        .ok_or_else(|| "Chrome cookie schema version is unavailable".to_owned())
}

fn query_cookies(database_path: &Path) -> Result<Vec<ChromeRow>, String> {
    let query = r#"
        SELECT host_key, top_frame_site_key, name, value, path,
               CAST(expires_utc AS TEXT) AS expires_utc,
               is_secure, is_httponly, has_expires, samesite,
               hex(encrypted_value) AS encrypted_hex
        FROM cookies
        ORDER BY host_key, path, name;
    "#;
    let output = sqlite_json(database_path, query)?;
    serde_json::from_str(&output).map_err(display_error)
}

fn command_secret(command: &str, arguments: &[&str]) -> Result<String, ()> {
    let output = Command::new(command)
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|_| ())?;
    if !output.status.success() {
        return Err(());
    }
    let secret = String::from_utf8(output.stdout).map_err(|_| ())?;
    let secret = secret.trim_end_matches(['\r', '\n']).to_owned();
    (!secret.is_empty()).then_some(secret).ok_or(())
}

fn service_account_token_path() -> Option<PathBuf> {
    let state_home = std::env::var_os("XDG_STATE_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
        })?;
    Some(state_home.join("1password/service-account.token"))
}

fn read_encryption_key(key_reference: &str) -> Result<String, String> {
    if let Some(token_path) = service_account_token_path()
        && let Ok(token) = fs::read_to_string(token_path)
        && !token.trim().is_empty()
    {
        let token = token.trim();
        let output = Command::new("op")
            .args(["read", key_reference])
            .env("OP_SERVICE_ACCOUNT_TOKEN", token)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output();
        if let Ok(output) = output
            && output.status.success()
            && let Ok(key) = String::from_utf8(output.stdout)
        {
            let key = key.trim_end_matches(['\r', '\n']).to_owned();
            if !key.is_empty() {
                return Ok(key);
            }
        }
    }

    let output = Command::new("op")
        .args(["read", key_reference])
        .env_remove("OP_SERVICE_ACCOUNT_TOKEN")
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .output()
        .map_err(|_| "1Password CLI is unavailable".to_owned())?;
    if !output.status.success() {
        return Err("1Password approval or access to the capsule key failed".to_owned());
    }
    let key = String::from_utf8(output.stdout)
        .map_err(|_| "1Password returned invalid UTF-8".to_owned())?;
    let key = key.trim_end_matches(['\r', '\n']).to_owned();
    if key.is_empty() {
        return Err("1Password returned an empty capsule key".to_owned());
    }
    Ok(key)
}

fn derive_chrome_key(password: &[u8]) -> [u8; 16] {
    let mut key = [0_u8; 16];
    pbkdf2_hmac::<Sha1>(password, b"saltysalt", 1, &mut key);
    key
}

fn decrypt_chrome_cookie(
    row: &ChromeRow,
    schema_version: i64,
    linux_secret: &str,
) -> Result<String, String> {
    if !row.value.is_empty() {
        return Ok(row.value.clone());
    }
    let encrypted = hex::decode(&row.encrypted_hex)
        .map_err(|_| "Chrome cookie ciphertext is malformed".to_owned())?;
    if encrypted.len() < 4 {
        return Err("Chrome cookie ciphertext is too short".to_owned());
    }
    let prefix = &encrypted[..3];
    let password = match prefix {
        b"v10" => b"peanuts".as_slice(),
        b"v11" => linux_secret.as_bytes(),
        _ => return Err("Chrome uses an unsupported cookie encryption prefix".to_owned()),
    };
    let key = derive_chrome_key(password);
    let plaintext = Aes128CbcDec::new(&key.into(), &[0x20_u8; 16].into())
        .decrypt_padded_vec_mut::<Pkcs7>(&encrypted[3..])
        .map_err(|_| "Chrome cookie decryption failed".to_owned())?;
    let value = if schema_version >= 24 {
        if plaintext.len() < 32 || plaintext[..32] != Sha256::digest(row.host_key.as_bytes())[..] {
            return Err("Chrome cookie host digest validation failed".to_owned());
        }
        &plaintext[32..]
    } else {
        &plaintext
    };
    String::from_utf8(value.to_vec()).map_err(|_| "Chrome cookie value is not UTF-8".to_owned())
}

fn chrome_timestamp_to_unix_seconds(value: &str, has_expires: bool) -> Result<i64, String> {
    if !has_expires {
        return Ok(-1);
    }
    let microseconds = value
        .parse::<i64>()
        .map_err(|_| "Invalid Chrome cookie expiry".to_owned())?;
    Ok(microseconds / 1_000_000 - CHROME_EPOCH_OFFSET_SECONDS)
}

fn map_same_site(value: i64) -> Result<Option<String>, String> {
    match value {
        -1 => Ok(None),
        0 => Ok(Some("None".to_owned())),
        1 => Ok(Some("Lax".to_owned())),
        2 => Ok(Some("Strict".to_owned())),
        _ => Err(format!("Unsupported Chrome SameSite value: {value}")),
    }
}

fn domain_matches(cookie_domain: &str, hostname: &str) -> bool {
    let cookie_domain = cookie_domain.trim_start_matches('.').to_ascii_lowercase();
    let hostname = hostname.to_ascii_lowercase();
    hostname == cookie_domain || hostname.ends_with(&format!(".{cookie_domain}"))
}

fn cookie_matches_origin(cookie: &Cookie, origin: &Url, now_seconds: i64) -> bool {
    domain_matches(&cookie.domain, origin.host_str().unwrap_or_default())
        && (!cookie.secure || origin.scheme() == "https")
        && (cookie.expires == -1 || cookie.expires > now_seconds)
}

fn select_cookies(cookies: &[Cookie], origins: &[Url]) -> Vec<Cookie> {
    let now_seconds = Utc::now().timestamp();
    cookies
        .iter()
        .filter(|cookie| {
            origins
                .iter()
                .any(|origin| cookie_matches_origin(cookie, origin, now_seconds))
        })
        .cloned()
        .collect()
}

fn derive_capsule_key(secret: &[u8], salt: &[u8], info: &[u8]) -> Result<[u8; 32], String> {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), secret);
    let mut key = [0_u8; 32];
    hkdf.expand(info, &mut key)
        .map_err(|_| "Capsule key derivation failed".to_owned())?;
    Ok(key)
}

fn encrypt_capsule(capsule: &Capsule, secret: &[u8]) -> Result<Envelope, String> {
    let mut salt = [0_u8; 16];
    let mut nonce = [0_u8; 12];
    getrandom::fill(&mut salt).map_err(display_error)?;
    getrandom::fill(&mut nonce).map_err(display_error)?;
    let key = derive_capsule_key(secret, &salt, KDF_INFO.as_bytes())?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| "Capsule cipher initialization failed".to_owned())?;
    let aad = format!("{ENVELOPE_FORMAT}:{VERSION}:{CIPHER}");
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &serde_json::to_vec(capsule).map_err(display_error)?,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| "Capsule encryption failed".to_owned())?;
    let tag_offset = encrypted
        .len()
        .checked_sub(16)
        .ok_or_else(|| "Capsule ciphertext is malformed".to_owned())?;
    Ok(Envelope {
        format: ENVELOPE_FORMAT.to_owned(),
        version: VERSION,
        cipher: CIPHER.to_owned(),
        kdf: EnvelopeKdf {
            name: KDF_NAME.to_owned(),
            salt: BASE64.encode(salt),
            info: KDF_INFO.to_owned(),
        },
        nonce: BASE64.encode(nonce),
        tag: BASE64.encode(&encrypted[tag_offset..]),
        ciphertext: BASE64.encode(&encrypted[..tag_offset]),
    })
}

fn decrypt_capsule(envelope: &Envelope, secret: &[u8]) -> Result<Capsule, String> {
    if envelope.format != ENVELOPE_FORMAT
        || envelope.version != VERSION
        || envelope.cipher != CIPHER
        || envelope.kdf.name != KDF_NAME
        || envelope.kdf.info != KDF_INFO
    {
        return Err("Unsupported capsule format or encryption parameters".to_owned());
    }
    let salt = BASE64
        .decode(&envelope.kdf.salt)
        .map_err(|_| "Capsule salt is malformed".to_owned())?;
    let nonce = BASE64
        .decode(&envelope.nonce)
        .map_err(|_| "Capsule nonce is malformed".to_owned())?;
    let ciphertext = BASE64
        .decode(&envelope.ciphertext)
        .map_err(|_| "Capsule ciphertext is malformed".to_owned())?;
    let tag = BASE64
        .decode(&envelope.tag)
        .map_err(|_| "Capsule tag is malformed".to_owned())?;
    if nonce.len() != 12 || tag.len() != 16 {
        return Err("Capsule nonce or tag has an invalid length".to_owned());
    }
    let key = derive_capsule_key(secret, &salt, envelope.kdf.info.as_bytes())?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| "Capsule cipher initialization failed".to_owned())?;
    let mut encrypted = ciphertext;
    encrypted.extend_from_slice(&tag);
    let aad = format!(
        "{}:{}:{}",
        envelope.format, envelope.version, envelope.cipher
    );
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &encrypted,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| "Capsule authentication or decryption failed".to_owned())?;
    let capsule: Capsule = serde_json::from_slice(&plaintext)
        .map_err(|_| "Decrypted capsule is malformed".to_owned())?;
    if capsule.format != CAPSULE_FORMAT || capsule.version != VERSION {
        return Err("Unsupported decrypted capsule format".to_owned());
    }
    Ok(capsule)
}

fn default_output_path(target: &Url) -> Result<PathBuf, String> {
    let user_home = env::var_os("HOME").ok_or_else(|| "HOME is unavailable".to_owned())?;
    let hostname = target.host_str().unwrap_or("website").replace(
        |character: char| {
            !character.is_ascii_alphanumeric() && character != '-' && character != '.'
        },
        "-",
    );
    let timestamp = Utc::now().format("%Y-%m-%dT%H%M%SZ");
    Ok(PathBuf::from(user_home)
        .join(".local/share/codex/browser-session-capsules")
        .join(format!("{hostname}-{timestamp}.cookies.enc.json")))
}

fn write_private_new_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Capsule output path has no parent".to_owned())?;
    let parent_existed = parent.exists();
    fs::create_dir_all(parent).map_err(display_error)?;
    if !parent_existed {
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(display_error)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("Unable to create capsule output: {error}"))?;
    file.write_all(contents).map_err(display_error)?;
    file.write_all(b"\n").map_err(display_error)
}

fn metadata_json(event: &str, origins: &[Url], cookies: &[Cookie], capsule: &Capsule) -> Value {
    json!({
        "event": event,
        "targetOrigins": origins.iter().map(|origin| origin.origin().ascii_serialization()).collect::<Vec<_>>(),
        "matchingCookieCount": cookies.len(),
        "partitionedCookieCount": cookies.iter().filter(|cookie| cookie.partition_key.is_some()).count(),
        "capsuleCreatedAt": capsule.created_at,
    })
}

fn serve_request(
    stream: &mut TcpStream,
    request_token: &str,
    cookies: &[Cookie],
) -> Result<bool, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(display_error)?;
    let mut request = [0_u8; 4096];
    let count = stream.read(&mut request).map_err(display_error)?;
    let request = String::from_utf8_lossy(&request[..count]);
    let expected_prefix = format!("GET /{request_token} HTTP/1.");
    if !request.starts_with(&expected_prefix) {
        stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .map_err(display_error)?;
        return Ok(false);
    }
    let body = serde_json::to_vec(&json!({ "cookies": cookies })).map_err(display_error)?;
    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(headers.as_bytes())
        .map_err(display_error)?;
    stream.write_all(&body).map_err(display_error)?;
    stream.flush().map_err(display_error)?;
    Ok(true)
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
