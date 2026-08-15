#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
edition = "2024"

[dependencies]
anyhow = "1.0.100"
fs2 = "0.4.3"
libc = "0.2.177"
serde_json = "1.0.145"
---

use anyhow::{bail, Context, Result};
use fs2::FileExt;
use serde_json::{json, Value};
use std::collections::{HashSet, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

const BROKER_PROTOCOL: &str = "browser-auth-broker/1";
const DEFAULT_LEASE_IDLE_MS: u64 = 60_000;
const DEFAULT_WAIT_TIMEOUT_MS: u64 = 300_000;
const MAX_WAIT_TIMEOUT_MS: u64 = 3_600_000;
const DAEMON_EMPTY_GRACE_MS: u64 = 5_000;
const START_TIMEOUT_MS: u64 = 15_000;
const CONNECT_POLL_MS: u64 = 50;
const WAIT_POLL_MS: u64 = 250;

fn main() {
    let result = match std::env::args().nth(1).as_deref() {
        Some("__daemon") => run_daemon(),
        Some("__fake_upstream") => run_fake_upstream(),
        Some("__integration_test") => run_integration_test(),
        _ => run_stdio_wrapper(),
    };

    if let Err(error) = result {
        eprintln!("browser_auth broker: {error:#}");
        std::process::exit(1);
    }
}

#[derive(Clone)]
struct BrokerPaths {
    socket: PathBuf,
    state: PathBuf,
    start_lock: PathBuf,
    log: PathBuf,
    upstream_log: PathBuf,
}

impl BrokerPaths {
    fn resolve() -> Result<Self> {
        let base = if let Some(runtime) = std::env::var_os("CODEX_BROWSER_AUTH_RUNTIME_DIR") {
            PathBuf::from(runtime)
        } else if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
            PathBuf::from(runtime)
        } else {
            PathBuf::from(format!("/tmp/codex-browser-auth-{}", unsafe {
                libc::geteuid()
            }))
        };
        let dir = base.join("codex-browser-auth-broker");
        fs::create_dir_all(&dir).with_context(|| format!("Failed to create {}", dir.display()))?;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
            .with_context(|| format!("Failed to chmod {}", dir.display()))?;
        Ok(Self {
            socket: dir.join("broker.sock"),
            state: dir.join("state.json"),
            start_lock: dir.join("start.lock"),
            log: dir.join("broker.log"),
            upstream_log: dir.join("playwright-mcp.log"),
        })
    }
}

fn open_private_file(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
        .with_context(|| format!("Failed to open {}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("Failed to chmod {}", path.display()))?;
    Ok(file)
}

fn run_stdio_wrapper() -> Result<()> {
    let paths = BrokerPaths::resolve()?;
    let start_lock = open_private_file(&paths.start_lock)?;
    start_lock
        .lock_exclusive()
        .context("Failed to acquire browser_auth broker startup lock")?;

    let stream = match connect_ready(&paths.socket) {
        Ok(stream) => stream,
        Err(_) => match wait_for_stale_daemon(&paths)? {
            Some(stream) => stream,
            None => {
                remove_if_exists(&paths.socket)?;
                spawn_daemon(&paths)?;
                wait_for_daemon(&paths)?
            }
        },
    };
    drop(start_lock);
    bridge_stdio(stream)
}

fn connect_ready(socket: &Path) -> Result<UnixStream> {
    let stream = UnixStream::connect(socket)
        .with_context(|| format!("Failed to connect to {}", socket.display()))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .context("Failed to configure broker handshake timeout")?;
    let mut reader = BufReader::new(stream);
    let mut handshake = String::new();
    reader
        .read_line(&mut handshake)
        .context("Failed to read broker handshake")?;
    if handshake.trim_end() != BROKER_PROTOCOL {
        bail!("Unexpected browser_auth broker handshake");
    }
    let stream = reader.into_inner();
    stream
        .set_read_timeout(None)
        .context("Failed to clear broker handshake timeout")?;
    Ok(stream)
}

fn wait_for_stale_daemon(paths: &BrokerPaths) -> Result<Option<UnixStream>> {
    let Some(pid) = read_state_pid(&paths.state)? else {
        return Ok(None);
    };
    if !process_exists(pid) {
        remove_if_exists(&paths.state)?;
        return Ok(None);
    }

    let deadline = Instant::now() + Duration::from_millis(START_TIMEOUT_MS);
    while Instant::now() < deadline {
        if let Ok(stream) = connect_ready(&paths.socket) {
            return Ok(Some(stream));
        }
        if !process_exists(pid) {
            remove_if_exists(&paths.state)?;
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(CONNECT_POLL_MS));
    }
    bail!("Existing browser_auth broker process {pid} is alive but unreachable");
}

fn spawn_daemon(paths: &BrokerPaths) -> Result<()> {
    let executable = std::env::current_exe().context("Failed to resolve broker executable")?;
    let stdout = append_log(&paths.log)?;
    let stderr = stdout
        .try_clone()
        .context("Failed to clone broker log handle")?;
    let mut command = Command::new(executable);
    command
        .arg("__daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command
        .spawn()
        .context("Failed to spawn browser_auth broker daemon")?;
    Ok(())
}

fn wait_for_daemon(paths: &BrokerPaths) -> Result<UnixStream> {
    let deadline = Instant::now() + Duration::from_millis(START_TIMEOUT_MS);
    let mut last_error = None;
    while Instant::now() < deadline {
        match connect_ready(&paths.socket) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
        thread::sleep(Duration::from_millis(CONNECT_POLL_MS));
    }
    let suffix = last_error
        .map(|error| format!(": {error:#}"))
        .unwrap_or_default();
    bail!(
        "Timed out starting browser_auth broker{suffix}. See {}",
        paths.log.display()
    )
}

fn bridge_stdio(stream: UnixStream) -> Result<()> {
    let reader = stream
        .try_clone()
        .context("Failed to clone browser_auth broker connection")?;
    let writer = stream;
    let (done_tx, done_rx) = mpsc::channel();

    let reader_done = done_tx.clone();
    thread::spawn(move || {
        let result = copy_stream(reader, io::stdout().lock());
        let _ = reader_done.send(result);
    });

    thread::spawn(move || {
        let result = copy_stream(io::stdin().lock(), &writer);
        let _ = writer.shutdown(std::net::Shutdown::Write);
        let _ = done_tx.send(result);
    });

    done_rx
        .recv()
        .context("browser_auth broker bridge stopped unexpectedly")??;
    Ok(())
}

fn copy_stream(mut reader: impl Read, mut writer: impl Write) -> io::Result<()> {
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let bytes = reader.read(&mut buffer)?;
        if bytes == 0 {
            writer.flush()?;
            return Ok(());
        }
        writer.write_all(&buffer[..bytes])?;
        writer.flush()?;
    }
}

fn run_daemon() -> Result<()> {
    let paths = BrokerPaths::resolve()?;
    if paths.socket.exists() {
        bail!(
            "Refusing to replace live broker socket {}",
            paths.socket.display()
        );
    }

    // Publish our identity before the slower Playwright startup. If the wrapper
    // that spawned us disappears, another wrapper waits instead of starting a
    // competing extension client.
    write_state(&paths.state)?;
    let upstream = Upstream::start(&paths.upstream_log)?;
    let broker = Arc::new(Broker::new(upstream)?);
    let listener = UnixListener::bind(&paths.socket)
        .with_context(|| format!("Failed to bind {}", paths.socket.display()))?;
    fs::set_permissions(&paths.socket, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("Failed to chmod {}", paths.socket.display()))?;
    listener
        .set_nonblocking(true)
        .context("Failed to configure broker listener")?;

    let clients = Arc::new(AtomicUsize::new(0));
    let active_requests = Arc::new(AtomicUsize::new(0));
    let ever_connected = Arc::new(AtomicBool::new(false));
    let last_empty = Arc::new(Mutex::new(None::<Instant>));
    let next_client = AtomicU64::new(1);

    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream
                    .write_all(format!("{BROKER_PROTOCOL}\n").as_bytes())
                    .context("Failed to write broker handshake")?;
                stream.flush().context("Failed to flush broker handshake")?;
                let client_id = next_client.fetch_add(1, Ordering::Relaxed);
                clients.fetch_add(1, Ordering::SeqCst);
                ever_connected.store(true, Ordering::SeqCst);
                *lock_unpoisoned(&last_empty) = None;
                let broker = broker.clone();
                let clients = clients.clone();
                let active_requests = active_requests.clone();
                let last_empty = last_empty.clone();
                thread::spawn(move || {
                    handle_client(stream, client_id, broker, active_requests);
                    if clients.fetch_sub(1, Ordering::SeqCst) == 1 {
                        *lock_unpoisoned(&last_empty) = Some(Instant::now());
                    }
                });
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error).context("browser_auth broker accept failed"),
        }

        let should_stop = ever_connected.load(Ordering::SeqCst)
            && clients.load(Ordering::SeqCst) == 0
            && active_requests.load(Ordering::SeqCst) == 0
            && lock_unpoisoned(&last_empty).is_some_and(|empty_since| {
                empty_since.elapsed() >= Duration::from_millis(DAEMON_EMPTY_GRACE_MS)
            });
        if should_stop {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    drop(listener);
    remove_if_exists(&paths.socket)?;
    remove_if_exists(&paths.state)?;
    Ok(())
}

fn handle_client(
    stream: UnixStream,
    client_id: u64,
    broker: Arc<Broker>,
    active_requests: Arc<AtomicUsize>,
) {
    let alive = Arc::new(AtomicBool::new(true));
    let cancelled = Arc::new(Mutex::new(HashSet::<String>::new()));
    let writer = match stream.try_clone() {
        Ok(stream) => Arc::new(Mutex::new(stream)),
        Err(_) => return,
    };
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let Ok(line) = line else { break };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            let response = json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": {"code": -32700, "message": "Parse error"}
            });
            let _ = write_json_line(&writer, &response);
            continue;
        };

        if message.get("method").and_then(Value::as_str) == Some("notifications/cancelled") {
            if let Some(request_id) = message.pointer("/params/requestId") {
                lock_unpoisoned(&cancelled).insert(request_key(request_id));
                broker.lease.notify();
            }
            continue;
        }
        if message.get("id").is_none() {
            continue;
        }

        let broker = broker.clone();
        let writer = writer.clone();
        let alive = alive.clone();
        let cancelled = cancelled.clone();
        let active_requests = active_requests.clone();
        active_requests.fetch_add(1, Ordering::SeqCst);
        thread::spawn(move || {
            let _active = ActiveRequestGuard(active_requests);
            let response = broker.handle_request(client_id, &message, &alive, &cancelled);
            if let Some(response) = response {
                let _ = write_json_line(&writer, &response);
            }
        });
    }

    alive.store(false, Ordering::SeqCst);
    broker.lease.disconnect(client_id);
    broker.lease.notify();
}

struct ActiveRequestGuard(Arc<AtomicUsize>);

impl Drop for ActiveRequestGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn write_json_line(writer: &Mutex<UnixStream>, value: &Value) -> Result<()> {
    let mut writer = lock_unpoisoned(writer);
    serde_json::to_writer(&mut *writer, value).context("Failed to serialize MCP response")?;
    writer
        .write_all(b"\n")
        .context("Failed to terminate MCP response")?;
    writer.flush().context("Failed to flush MCP response")?;
    Ok(())
}

struct Broker {
    upstream: Mutex<Upstream>,
    initialize_result: Value,
    tools_result: Value,
    lease: LeaseManager,
}

impl Broker {
    fn new(mut upstream: Upstream) -> Result<Self> {
        let initialize_result = upstream.initialize()?;
        let mut tools_result = upstream.request_result("tools/list", json!({}))?;
        let tools = tools_result
            .get_mut("tools")
            .and_then(Value::as_array_mut)
            .context("Playwright MCP returned an invalid tools/list result")?;
        tools.push(wait_for_lock_tool());
        tools.push(unlock_tool());
        Ok(Self {
            upstream: Mutex::new(upstream),
            initialize_result,
            tools_result,
            lease: LeaseManager::new(lease_idle_timeout()),
        })
    }

    fn handle_request(
        &self,
        client_id: u64,
        request: &Value,
        alive: &AtomicBool,
        cancelled: &Mutex<HashSet<String>>,
    ) -> Option<Value> {
        let id = request.get("id")?.clone();
        let method = request
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let result = match method {
            "initialize" => Ok(self.initialize_result.clone()),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(self.tools_result.clone()),
            "tools/call" => self.handle_tool_call(client_id, request, alive, cancelled),
            _ => Err(format!("Unsupported MCP method: {method}")),
        };

        Some(match result {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err(message) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32603, "message": message}
            }),
        })
    }

    fn handle_tool_call(
        &self,
        client_id: u64,
        request: &Value,
        alive: &AtomicBool,
        cancelled: &Mutex<HashSet<String>>,
    ) -> std::result::Result<Value, String> {
        let name = request
            .pointer("/params/name")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing tool name".to_owned())?;
        let arguments = request
            .pointer("/params/arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        match name {
            "browser_auth_wait_for_lock" => {
                let timeout_ms = arguments
                    .get("timeoutMs")
                    .and_then(Value::as_u64)
                    .unwrap_or(DEFAULT_WAIT_TIMEOUT_MS)
                    .min(MAX_WAIT_TIMEOUT_MS);
                let request_id = request.get("id").map(request_key).unwrap_or_default();
                let acquired = self.lease.wait_for(
                    client_id,
                    Duration::from_millis(timeout_ms),
                    || {
                        !alive.load(Ordering::SeqCst)
                            || lock_unpoisoned(cancelled).contains(&request_id)
                    },
                );
                lock_unpoisoned(cancelled).remove(&request_id);
                match acquired {
                    WaitOutcome::Acquired => Ok(tool_text(json!({
                        "acquired": true,
                        "idleTimeoutMs": lease_idle_timeout().as_millis()
                    }))),
                    WaitOutcome::TimedOut => Ok(tool_error(
                        "Timed out waiting for browser_auth. Retry browser_auth_wait_for_lock.",
                    )),
                    WaitOutcome::Cancelled => Ok(tool_error("browser_auth lock wait was cancelled.")),
                }
            }
            "browser_auth_unlock" => match self.lease.unlock(client_id) {
                UnlockOutcome::Released => Ok(tool_text(json!({"released": true}))),
                UnlockOutcome::NotOwner => Ok(tool_text(json!({"released": false, "reason": "not-owner"}))),
                UnlockOutcome::InFlight => Ok(tool_error(
                    "Cannot unlock browser_auth while this session still has a browser tool call in flight.",
                )),
            },
            _ => {
                let lease = self.lease.begin_tool(client_id);
                if let BeginOutcome::Busy { retry_after_ms } = lease {
                    return Ok(tool_error(&format!(
                        "browser_auth is locked by another Codex session. Call browser_auth_wait_for_lock, then retry this tool. The current lease is eligible for inactivity expiry in about {retry_after_ms} ms."
                    )));
                }

                let result = lock_unpoisoned(&self.upstream)
                    .request_result("tools/call", json!({"name": name, "arguments": arguments}))
                    .map_err(|error| format!("Playwright MCP call failed: {error:#}"));
                self.lease.finish_tool(client_id);
                result
            }
        }
    }
}

fn wait_for_lock_tool() -> Value {
    json!({
        "name": "browser_auth_wait_for_lock",
        "title": "Wait for authenticated browser lock",
        "description": "Wait in FIFO order for exclusive access to browser_auth. Browser tool calls acquire the lock automatically when it is free and renew it. The lease expires after 60 seconds without browser activity, never while a tool call is in flight.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "timeoutMs": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": MAX_WAIT_TIMEOUT_MS,
                    "default": DEFAULT_WAIT_TIMEOUT_MS
                }
            },
            "additionalProperties": false
        }
    })
}

fn unlock_tool() -> Value {
    json!({
        "name": "browser_auth_unlock",
        "title": "Unlock authenticated browser",
        "description": "Release this Codex session's browser_auth lease. Call this as soon as authenticated browser work is complete.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }
    })
}

fn tool_text(payload: Value) -> Value {
    json!({
        "content": [{"type": "text", "text": payload.to_string()}],
        "isError": false
    })
}

fn tool_error(message: &str) -> Value {
    json!({
        "content": [{"type": "text", "text": message}],
        "isError": true
    })
}

#[derive(Debug)]
struct LeaseOwner {
    client_id: u64,
    last_activity: Instant,
    in_flight: usize,
    release_when_idle: bool,
}

#[derive(Debug, Default)]
struct LeaseState {
    owner: Option<LeaseOwner>,
    queue: VecDeque<u64>,
}

struct LeaseManager {
    state: Mutex<LeaseState>,
    changed: Condvar,
    idle_timeout: Duration,
}

#[derive(Debug, PartialEq, Eq)]
enum BeginOutcome {
    Acquired,
    Busy { retry_after_ms: u64 },
}

#[derive(Debug, PartialEq, Eq)]
enum WaitOutcome {
    Acquired,
    TimedOut,
    Cancelled,
}

#[derive(Debug, PartialEq, Eq)]
enum UnlockOutcome {
    Released,
    NotOwner,
    InFlight,
}

impl LeaseManager {
    fn new(idle_timeout: Duration) -> Self {
        Self {
            state: Mutex::new(LeaseState::default()),
            changed: Condvar::new(),
            idle_timeout,
        }
    }

    fn begin_tool(&self, client_id: u64) -> BeginOutcome {
        let mut state = lock_unpoisoned(&self.state);
        self.expire_if_idle(&mut state);
        if state
            .owner
            .as_ref()
            .is_some_and(|owner| owner.client_id == client_id)
        {
            let owner = state.owner.as_mut().expect("owner checked");
            owner.in_flight += 1;
            owner.last_activity = Instant::now();
            return BeginOutcome::Acquired;
        }
        let has_priority = state
            .queue
            .front()
            .is_none_or(|queued| *queued == client_id);
        if state.owner.is_none() && has_priority {
            if state.queue.front() == Some(&client_id) {
                state.queue.pop_front();
            }
            state.owner = Some(LeaseOwner {
                client_id,
                last_activity: Instant::now(),
                in_flight: 1,
                release_when_idle: false,
            });
            return BeginOutcome::Acquired;
        }
        BeginOutcome::Busy {
            retry_after_ms: self.retry_after_ms(&state),
        }
    }

    fn finish_tool(&self, client_id: u64) {
        let mut state = lock_unpoisoned(&self.state);
        let mut release = false;
        if let Some(owner) = state
            .owner
            .as_mut()
            .filter(|owner| owner.client_id == client_id)
        {
            owner.in_flight = owner.in_flight.saturating_sub(1);
            owner.last_activity = Instant::now();
            release = owner.in_flight == 0 && owner.release_when_idle;
        }
        if release {
            state.owner = None;
        }
        self.changed.notify_all();
    }

    fn wait_for<F>(&self, client_id: u64, timeout: Duration, cancelled: F) -> WaitOutcome
    where
        F: Fn() -> bool,
    {
        let deadline = Instant::now() + timeout;
        let mut state = lock_unpoisoned(&self.state);
        loop {
            self.expire_if_idle(&mut state);
            if cancelled() {
                remove_queued(&mut state.queue, client_id);
                self.changed.notify_all();
                return WaitOutcome::Cancelled;
            }
            if state
                .owner
                .as_ref()
                .is_some_and(|owner| owner.client_id == client_id)
            {
                state.owner.as_mut().expect("owner checked").last_activity = Instant::now();
                remove_queued(&mut state.queue, client_id);
                return WaitOutcome::Acquired;
            }
            if state.owner.is_none()
                && state
                    .queue
                    .front()
                    .is_none_or(|queued| *queued == client_id)
            {
                if state.queue.front() == Some(&client_id) {
                    state.queue.pop_front();
                }
                state.owner = Some(LeaseOwner {
                    client_id,
                    last_activity: Instant::now(),
                    in_flight: 0,
                    release_when_idle: false,
                });
                return WaitOutcome::Acquired;
            }
            if !state.queue.contains(&client_id) {
                state.queue.push_back(client_id);
            }
            let now = Instant::now();
            if now >= deadline {
                remove_queued(&mut state.queue, client_id);
                self.changed.notify_all();
                return WaitOutcome::TimedOut;
            }
            let until_deadline = deadline.saturating_duration_since(now);
            let until_expiry = state
                .owner
                .as_ref()
                .filter(|owner| owner.in_flight == 0)
                .map(|owner| {
                    self.idle_timeout
                        .saturating_sub(owner.last_activity.elapsed())
                })
                .unwrap_or(Duration::from_millis(WAIT_POLL_MS));
            let sleep_for = until_deadline
                .min(until_expiry.max(Duration::from_millis(1)))
                .min(Duration::from_millis(WAIT_POLL_MS));
            let (next, _) = self
                .changed
                .wait_timeout(state, sleep_for)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state = next;
        }
    }

    fn unlock(&self, client_id: u64) -> UnlockOutcome {
        let mut state = lock_unpoisoned(&self.state);
        let Some(owner) = state.owner.as_ref() else {
            remove_queued(&mut state.queue, client_id);
            return UnlockOutcome::NotOwner;
        };
        if owner.client_id != client_id {
            remove_queued(&mut state.queue, client_id);
            return UnlockOutcome::NotOwner;
        }
        if owner.in_flight > 0 {
            return UnlockOutcome::InFlight;
        }
        state.owner = None;
        remove_queued(&mut state.queue, client_id);
        self.changed.notify_all();
        UnlockOutcome::Released
    }

    fn disconnect(&self, client_id: u64) {
        let mut state = lock_unpoisoned(&self.state);
        remove_queued(&mut state.queue, client_id);
        if let Some(owner) = state
            .owner
            .as_mut()
            .filter(|owner| owner.client_id == client_id)
        {
            if owner.in_flight == 0 {
                state.owner = None;
            } else {
                owner.release_when_idle = true;
            }
        }
        self.changed.notify_all();
    }

    fn notify(&self) {
        self.changed.notify_all();
    }

    fn expire_if_idle(&self, state: &mut LeaseState) {
        let expired = state.owner.as_ref().is_some_and(|owner| {
            owner.in_flight == 0 && owner.last_activity.elapsed() >= self.idle_timeout
        });
        if expired {
            state.owner = None;
            self.changed.notify_all();
        }
    }

    fn retry_after_ms(&self, state: &LeaseState) -> u64 {
        state
            .owner
            .as_ref()
            .filter(|owner| owner.in_flight == 0)
            .map(|owner| {
                self.idle_timeout
                    .saturating_sub(owner.last_activity.elapsed())
                    .as_millis()
                    .min(u64::MAX as u128) as u64
            })
            .unwrap_or(self.idle_timeout.as_millis().min(u64::MAX as u128) as u64)
    }
}

fn remove_queued(queue: &mut VecDeque<u64>, client_id: u64) {
    queue.retain(|queued| *queued != client_id);
}

struct Upstream {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Upstream {
    fn start(log_path: &Path) -> Result<Self> {
        let stderr = append_log(log_path)?;
        let mut process = if std::env::var_os("CODEX_BROWSER_AUTH_FAKE_UPSTREAM").is_some() {
            let mut process = Command::new(std::env::current_exe()?);
            process.arg("__fake_upstream");
            process
        } else {
            let command = std::env::var("CODEX_BROWSER_AUTH_MCP_COMMAND")
                .unwrap_or_else(|_| "pnpx".to_owned());
            let mut process = Command::new(command);
            process.args(["@playwright/mcp@latest", "--extension"]);
            process
        };
        process
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr));
        unsafe {
            process.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = process.spawn().context("Failed to spawn Playwright MCP")?;
        let stdin = child
            .stdin
            .take()
            .context("Playwright MCP stdin is unavailable")?;
        let stdout = child
            .stdout
            .take()
            .context("Playwright MCP stdout is unavailable")?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
        })
    }

    fn initialize(&mut self) -> Result<Value> {
        let result = self.request_result(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "Codex browser_auth broker", "version": "1.0.0"}
            }),
        )?;
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))?;
        Ok(result)
    }

    fn request_result(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))?;

        loop {
            let mut line = String::new();
            let bytes = self
                .stdout
                .read_line(&mut line)
                .context("Failed to read Playwright MCP response")?;
            if bytes == 0 {
                bail!("Playwright MCP exited unexpectedly");
            }
            let message: Value =
                serde_json::from_str(&line).context("Playwright MCP emitted invalid JSON")?;
            if message.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(error) = message.get("error") {
                    bail!("Playwright MCP returned {error}");
                }
                return message
                    .get("result")
                    .cloned()
                    .context("Playwright MCP response is missing result");
            }
            if message.get("id").is_some() && message.get("method").is_some() {
                self.answer_server_request(&message)?;
            }
        }
    }

    fn answer_server_request(&mut self, request: &Value) -> Result<()> {
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let response = match method {
            "roots/list" => json!({"jsonrpc": "2.0", "id": id, "result": {"roots": []}}),
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": format!("Unsupported broker client method: {method}")}
            }),
        };
        self.write_message(&response)
    }

    fn write_message(&mut self, message: &Value) -> Result<()> {
        serde_json::to_writer(&mut self.stdin, message)
            .context("Failed to serialize Playwright MCP request")?;
        self.stdin
            .write_all(b"\n")
            .context("Failed to terminate Playwright MCP request")?;
        self.stdin
            .flush()
            .context("Failed to flush Playwright MCP request")?;
        Ok(())
    }
}

impl Drop for Upstream {
    fn drop(&mut self) {
        let pid = self.child.id() as i32;
        unsafe {
            libc::kill(-pid, libc::SIGTERM);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => thread::sleep(Duration::from_millis(25)),
                Err(_) => break,
            }
        }
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
        let _ = self.child.wait();
    }
}

fn append_log(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("Failed to open {}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("Failed to chmod {}", path.display()))?;
    Ok(file)
}

fn write_state(path: &Path) -> Result<()> {
    let payload = json!({
        "protocol": BROKER_PROTOCOL,
        "pid": std::process::id()
    });
    let temp_path = path.with_file_name(format!("state.{}.tmp", std::process::id()));
    remove_if_exists(&temp_path)?;
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .with_context(|| format!("Failed to create {}", temp_path.display()))?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Failed to chmod {}", temp_path.display()))?;
        writeln!(file, "{payload}")
            .with_context(|| format!("Failed to write {}", temp_path.display()))?;
        file.sync_all()
            .with_context(|| format!("Failed to sync {}", temp_path.display()))?;
        fs::hard_link(&temp_path, path)
            .with_context(|| format!("Failed to claim daemon state {}", path.display()))?;
        Ok(())
    })();
    let cleanup = remove_if_exists(&temp_path);
    result.and(cleanup)
}

fn read_state_pid(path: &Path) -> Result<Option<i32>> {
    if !path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let state: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(state
        .get("pid")
        .and_then(Value::as_i64)
        .map(|pid| pid as i32))
}

fn process_exists(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }
    let result = unsafe { libc::kill(pid, 0) };
    result == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("Failed to remove {}", path.display())),
    }
}

fn lease_idle_timeout() -> Duration {
    let milliseconds = std::env::var("CODEX_BROWSER_AUTH_LOCK_IDLE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_LEASE_IDLE_MS);
    Duration::from_millis(milliseconds.max(1))
}

fn request_key(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn run_fake_upstream() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.context("Failed to read fake upstream request")?;
        let message: Value = serde_json::from_str(&line).context("Invalid fake upstream JSON")?;
        let Some(id) = message.get("id").cloned() else {
            continue;
        };
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let result = match method {
            "initialize" => json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "fake-playwright", "version": "1.0.0"}
            }),
            "tools/list" => json!({
                "tools": [{
                    "name": "browser_navigate",
                    "description": "Fake browser navigation",
                    "inputSchema": {"type": "object", "properties": {}}
                }]
            }),
            "tools/call" => json!({
                "content": [{
                    "type": "text",
                    "text": message.pointer("/params/name").and_then(Value::as_str).unwrap_or_default()
                }],
                "isError": false
            }),
            _ => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {"code": -32601, "message": "Unsupported fake method"}
                });
                serde_json::to_writer(&mut stdout, &response)?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
                continue;
            }
        };
        let response = json!({"jsonrpc": "2.0", "id": id, "result": result});
        serde_json::to_writer(&mut stdout, &response)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }
    Ok(())
}

struct ProbeClient {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl ProbeClient {
    fn spawn(executable: &Path, runtime_dir: &Path) -> Result<Self> {
        let mut child = Command::new(executable)
            .env("CODEX_BROWSER_AUTH_RUNTIME_DIR", runtime_dir)
            .env("CODEX_BROWSER_AUTH_FAKE_UPSTREAM", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn broker probe client")?;
        let stdin = child.stdin.take().context("Probe stdin unavailable")?;
        let stdout = child.stdout.take().context("Probe stdout unavailable")?;
        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
            next_id: 1,
        })
    }

    fn initialize(&mut self) -> Result<()> {
        self.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "probe", "version": "1.0.0"}
            }),
        )?;
        self.notify("notifications/initialized", json!({}))
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        self.write(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        loop {
            let mut line = String::new();
            if self.stdout.read_line(&mut line)? == 0 {
                bail!("Probe broker connection closed before response");
            }
            let response: Value = serde_json::from_str(&line)?;
            if response.get("id").and_then(Value::as_u64) == Some(id) {
                return Ok(response);
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        self.write(&json!({"jsonrpc": "2.0", "method": method, "params": params}))
    }

    fn write(&mut self, message: &Value) -> Result<()> {
        let stdin = self.stdin.as_mut().context("Probe stdin is closed")?;
        serde_json::to_writer(&mut *stdin, message)?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        Ok(())
    }

    fn stop(mut self) -> Result<()> {
        self.stdin.take();
        let status = self
            .child
            .wait()
            .context("Failed to wait for probe wrapper")?;
        if !status.success() {
            bail!("Probe wrapper exited with {status}");
        }
        Ok(())
    }
}

fn run_integration_test() -> Result<()> {
    let executable = std::env::current_exe().context("Failed to resolve broker test executable")?;
    let runtime_dir = PathBuf::from(format!(
        "/tmp/codex-browser-auth-broker-selftest-{}",
        std::process::id()
    ));
    if runtime_dir.exists() {
        fs::remove_dir_all(&runtime_dir)
            .with_context(|| format!("Failed to clear {}", runtime_dir.display()))?;
    }
    fs::create_dir_all(&runtime_dir)?;

    let mut first = ProbeClient::spawn(&executable, &runtime_dir)?;
    let mut second = ProbeClient::spawn(&executable, &runtime_dir)?;
    first.initialize()?;
    second.initialize()?;

    let tools = first.request("tools/list", json!({}))?;
    let names: Vec<&str> = tools
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .context("Probe tools/list result missing")?
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect();
    if !names.contains(&"browser_navigate")
        || !names.contains(&"browser_auth_wait_for_lock")
        || !names.contains(&"browser_auth_unlock")
    {
        bail!("Broker tool projection is incomplete: {names:?}");
    }

    let first_call = first.request(
        "tools/call",
        json!({"name": "browser_navigate", "arguments": {}}),
    )?;
    if first_call
        .pointer("/result/isError")
        .and_then(Value::as_bool)
        != Some(false)
    {
        bail!("First probe client failed to acquire browser lease");
    }
    let blocked = second.request(
        "tools/call",
        json!({"name": "browser_navigate", "arguments": {}}),
    )?;
    if blocked.pointer("/result/isError").and_then(Value::as_bool) != Some(true) {
        bail!("Second probe client bypassed browser lease");
    }

    let waiter = thread::spawn(move || -> Result<(ProbeClient, Value)> {
        let result = second.request(
            "tools/call",
            json!({
                "name": "browser_auth_wait_for_lock",
                "arguments": {"timeoutMs": 2_000}
            }),
        )?;
        Ok((second, result))
    });
    thread::sleep(Duration::from_millis(50));
    let unlocked = first.request(
        "tools/call",
        json!({"name": "browser_auth_unlock", "arguments": {}}),
    )?;
    if unlocked.pointer("/result/isError").and_then(Value::as_bool) != Some(false) {
        bail!("First probe client failed to unlock");
    }
    let (mut second, waited) = waiter
        .join()
        .map_err(|_| anyhow::anyhow!("Lock waiter panicked"))??;
    if waited.pointer("/result/isError").and_then(Value::as_bool) != Some(false) {
        bail!("Second probe client failed to acquire queued lease");
    }
    let second_call = second.request(
        "tools/call",
        json!({"name": "browser_navigate", "arguments": {}}),
    )?;
    if second_call
        .pointer("/result/isError")
        .and_then(Value::as_bool)
        != Some(false)
    {
        bail!("Queued lease did not authorize second probe client");
    }
    second.request(
        "tools/call",
        json!({"name": "browser_auth_unlock", "arguments": {}}),
    )?;

    first.stop()?;
    second.stop()?;
    let socket = runtime_dir
        .join("codex-browser-auth-broker")
        .join("broker.sock");
    let deadline = Instant::now() + Duration::from_secs(8);
    while socket.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(50));
    }
    if socket.exists() {
        bail!("Broker daemon did not stop after all clients disconnected");
    }
    fs::remove_dir_all(&runtime_dir)
        .with_context(|| format!("Failed to clean {}", runtime_dir.display()))?;
    println!("browser_auth broker integration test passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_acquire_is_exclusive() {
        let leases = LeaseManager::new(Duration::from_secs(1));
        assert_eq!(leases.begin_tool(1), BeginOutcome::Acquired);
        assert!(matches!(leases.begin_tool(2), BeginOutcome::Busy { .. }));
        leases.finish_tool(1);
        assert_eq!(leases.unlock(1), UnlockOutcome::Released);
        assert_eq!(leases.begin_tool(2), BeginOutcome::Acquired);
    }

    #[test]
    fn waiters_are_fifo() {
        let leases = Arc::new(LeaseManager::new(Duration::from_secs(1)));
        assert_eq!(leases.begin_tool(1), BeginOutcome::Acquired);
        leases.finish_tool(1);
        let first = {
            let leases = leases.clone();
            thread::spawn(move || leases.wait_for(2, Duration::from_secs(1), || false))
        };
        thread::sleep(Duration::from_millis(20));
        let second = {
            let leases = leases.clone();
            thread::spawn(move || leases.wait_for(3, Duration::from_secs(1), || false))
        };
        thread::sleep(Duration::from_millis(20));
        assert_eq!(leases.unlock(1), UnlockOutcome::Released);
        assert_eq!(first.join().unwrap(), WaitOutcome::Acquired);
        assert_eq!(leases.unlock(2), UnlockOutcome::Released);
        assert_eq!(second.join().unwrap(), WaitOutcome::Acquired);
    }

    #[test]
    fn in_flight_call_cannot_expire() {
        let leases = LeaseManager::new(Duration::from_millis(25));
        assert_eq!(leases.begin_tool(1), BeginOutcome::Acquired);
        thread::sleep(Duration::from_millis(50));
        assert!(matches!(leases.begin_tool(2), BeginOutcome::Busy { .. }));
        leases.finish_tool(1);
        thread::sleep(Duration::from_millis(30));
        assert_eq!(leases.begin_tool(2), BeginOutcome::Acquired);
    }

    #[test]
    fn disconnect_releases_after_in_flight_finishes() {
        let leases = LeaseManager::new(Duration::from_secs(1));
        assert_eq!(leases.begin_tool(1), BeginOutcome::Acquired);
        leases.disconnect(1);
        assert!(matches!(leases.begin_tool(2), BeginOutcome::Busy { .. }));
        leases.finish_tool(1);
        assert_eq!(leases.begin_tool(2), BeginOutcome::Acquired);
    }

    #[test]
    fn idle_owner_expires_for_waiter() {
        let leases = LeaseManager::new(Duration::from_millis(20));
        assert_eq!(leases.begin_tool(1), BeginOutcome::Acquired);
        leases.finish_tool(1);
        assert_eq!(
            leases.wait_for(2, Duration::from_secs(1), || false),
            WaitOutcome::Acquired
        );
    }
}
