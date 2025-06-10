use axum::{
    Json as AxumJson, Router,
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    process::Stdio,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::Mutex,
    time::{Duration, timeout},
};

// --- Configuration constants and defaults ---
const DEFAULT_MCP_SERVERS_DIR: &str = "/app/mcp-servers";
const DEFAULT_RESPONSE_TIMEOUT_SECS: u64 = 30;
const DEFAULT_PROCESS_INIT_WAIT_SECS: u64 = 2;
const DEFAULT_CONFIG_FILE: &str = "mcp_servers.config.json";
const DEFAULT_SERVER_NAME: &str = "readability";
const DEFAULT_PORT: &str = "3000";
const DEFAULT_HOST: &str = "0.0.0.0";

// --- Configuration structures ---
#[derive(Clone, Debug)]
struct ServerConfig {
    mcp_servers_dir: String,
    response_timeout_secs: u64,
    process_init_wait_secs: u64,
    supported_languages: Vec<String>,
    supported_server_types: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            mcp_servers_dir: env::var("MCP_SERVERS_DIR")
                .unwrap_or_else(|_| DEFAULT_MCP_SERVERS_DIR.to_string()),
            response_timeout_secs: env::var("RESPONSE_TIMEOUT_SECS")
                .unwrap_or_else(|_| DEFAULT_RESPONSE_TIMEOUT_SECS.to_string())
                .parse()
                .unwrap_or(DEFAULT_RESPONSE_TIMEOUT_SECS),
            process_init_wait_secs: env::var("PROCESS_INIT_WAIT_SECS")
                .unwrap_or_else(|_| DEFAULT_PROCESS_INIT_WAIT_SECS.to_string())
                .parse()
                .unwrap_or(DEFAULT_PROCESS_INIT_WAIT_SECS),
            supported_languages: env::var("SUPPORTED_LANGUAGES")
                .unwrap_or_else(|_| "node,python".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            supported_server_types: env::var("SUPPORTED_SERVER_TYPES")
                .unwrap_or_else(|_| "github".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
struct AuthConfig {
    api_key: Option<String>,
    enabled: bool,
}

#[derive(Serialize)]
struct AuthError {
    error: String,
    message: String,
}

#[derive(Deserialize, Debug, Clone)]
struct McpServerConfig {
    #[serde(rename = "type")]
    server_type: String,
    repository: Option<String>,
    language: String,
    entrypoint: String,
    description: Option<String>,
    install_command: Option<String>,
}

type McpServersConfig = HashMap<String, McpServerConfig>;

// --- Utility functions for enhanced logging ---
fn get_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

fn log_with_timestamp(level: &str, module: &str, message: &str) {
    let timestamp = get_timestamp();
    println!("[{}] [{}] [{}] {}", timestamp, level, module, message);
}

macro_rules! log_debug {
    ($module:expr, $($arg:tt)*) => {
        log_with_timestamp("DEBUG", $module, &format!($($arg)*));
    };
}

macro_rules! log_info {
    ($module:expr, $($arg:tt)*) => {
        log_with_timestamp("INFO", $module, &format!($($arg)*));
    };
}

macro_rules! log_warn {
    ($module:expr, $($arg:tt)*) => {
        log_with_timestamp("WARN", $module, &format!($($arg)*));
    };
}

macro_rules! log_error {
    ($module:expr, $($arg:tt)*) => {
        log_with_timestamp("ERROR", $module, &format!($($arg)*));
    };
}

// --- MCP Process management ---
struct McpServerProcess {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    process_id: Option<u32>,
    start_time: Instant,
    request_count: u64,
    last_activity: Instant,
    child_handle: Arc<Mutex<Option<Child>>>,
    config: ServerConfig,
}

impl McpServerProcess {
    async fn is_process_alive(&self) -> bool {
        if let Ok(mut child_guard) = self.child_handle.try_lock() {
            if let Some(ref mut child) = child_guard.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        log_warn!("MCP_PROCESS", "Process has exited with status: {}", status);
                        false
                    }
                    Ok(None) => {
                        log_debug!("MCP_PROCESS", "Process is still running");
                        true
                    }
                    Err(e) => {
                        log_error!("MCP_PROCESS", "Failed to check process status: {}", e);
                        false
                    }
                }
            } else {
                log_warn!("MCP_PROCESS", "No child process handle available");
                false
            }
        } else {
            log_warn!("MCP_PROCESS", "Cannot acquire child process lock");
            true // Assume alive if we can't check
        }
    }

    async fn query(&mut self, request: &McpRequest) -> Result<McpResponse, String> {
        let query_start = Instant::now();
        self.request_count += 1;

        log_debug!(
            "MCP_PROCESS",
            "Query #{} started - PID: {:?}",
            self.request_count,
            self.process_id
        );

        // Check if process is still alive before attempting communication
        if !self.is_process_alive().await {
            log_error!(
                "MCP_PROCESS",
                "Cannot send query: MCP process has terminated"
            );
            return Err("MCP process has terminated".to_string());
        }

        log_debug!("MCP_PROCESS", "Request details: {:?}", request);
        log_debug!(
            "MCP_PROCESS",
            "Time since last activity: {:?}",
            self.last_activity.elapsed()
        );

        // Prepare request data
        let request_data = request.command.clone() + "\n";
        let request_bytes = request_data.as_bytes();

        log_debug!(
            "MCP_PROCESS",
            "Sending {} bytes to stdin",
            request_bytes.len()
        );
        log_debug!(
            "MCP_PROCESS",
            "Request content: {}",
            request.command.chars().take(100).collect::<String>()
        );

        // Send request with detailed error tracking
        match self.stdin.write_all(request_bytes).await {
            Ok(_) => {
                log_debug!("MCP_PROCESS", "Successfully wrote request to stdin");
            }
            Err(e) => {
                log_error!(
                    "MCP_PROCESS",
                    "Failed to write to stdin: {} (errno: {:?})",
                    e,
                    e.raw_os_error()
                );
                return Err(format!(
                    "Failed to write to MCP stdin: {} (errno: {:?})",
                    e,
                    e.raw_os_error()
                ));
            }
        }

        // Flush with error tracking
        match self.stdin.flush().await {
            Ok(_) => {
                log_debug!("MCP_PROCESS", "Successfully flushed stdin");
            }
            Err(e) => {
                log_error!(
                    "MCP_PROCESS",
                    "Failed to flush stdin: {} (errno: {:?})",
                    e,
                    e.raw_os_error()
                );
                return Err(format!(
                    "Failed to flush MCP stdin: {} (errno: {:?})",
                    e,
                    e.raw_os_error()
                ));
            }
        }

        log_debug!(
            "MCP_PROCESS",
            "Request sent, waiting for response (timeout: {}s)",
            self.config.response_timeout_secs
        );

        // Read response with enhanced timeout and error tracking
        let response_result = timeout(
            Duration::from_secs(self.config.response_timeout_secs),
            async {
                let mut response_line = String::new();
                let read_start = Instant::now();

                log_debug!("MCP_PROCESS", "Starting to read response from stdout");

                match self.stdout.read_line(&mut response_line).await {
                    Ok(0) => {
                        log_warn!("MCP_PROCESS", "MCP server closed connection (read 0 bytes)");
                        Err("MCP server closed connection".to_string())
                    }
                    Ok(bytes_read) => {
                        log_debug!(
                            "MCP_PROCESS",
                            "Read {} bytes in {:?}",
                            bytes_read,
                            read_start.elapsed()
                        );

                        let response = response_line.trim();
                        if response.is_empty() {
                            log_warn!("MCP_PROCESS", "Received empty response");
                            Err("Empty response from MCP server".to_string())
                        } else {
                            log_debug!(
                                "MCP_PROCESS",
                                "Response content: {}",
                                response.chars().take(200).collect::<String>()
                            );
                            Ok(McpResponse {
                                result: response.to_string(),
                            })
                        }
                    }
                    Err(e) => {
                        log_error!(
                            "MCP_PROCESS",
                            "Failed to read response: {} (errno: {:?})",
                            e,
                            e.raw_os_error()
                        );
                        Err(format!(
                            "Failed to read response: {} (errno: {:?})",
                            e,
                            e.raw_os_error()
                        ))
                    }
                }
            },
        )
        .await;

        // Update activity tracking
        self.last_activity = Instant::now();

        match response_result {
            Ok(result) => {
                log_info!(
                    "MCP_PROCESS",
                    "Query #{} completed successfully in {:?}",
                    self.request_count,
                    query_start.elapsed()
                );
                result
            }
            Err(_) => {
                log_error!(
                    "MCP_PROCESS",
                    "Query #{} timed out after {:?}",
                    self.request_count,
                    query_start.elapsed()
                );
                Err("MCP server timeout".to_string())
            }
        }
    }

    fn get_stats(&self) -> String {
        format!(
            "PID: {:?}, Uptime: {:?}, Requests: {}, Last activity: {:?} ago",
            self.process_id,
            self.start_time.elapsed(),
            self.request_count,
            self.last_activity.elapsed()
        )
    }
}

// --- Request/Response structures ---
#[derive(Serialize, Deserialize, Debug)]
struct McpRequest {
    command: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct McpResponse {
    result: String,
}

// --- Setup functions ---
async fn setup_mcp_server(
    server_name: &str,
    config: &McpServerConfig,
    server_config: &ServerConfig,
) -> Result<(), String> {
    log_debug!("SETUP", "Setting up MCP server: {}", server_name);

    if !server_config
        .supported_server_types
        .contains(&config.server_type)
    {
        log_error!(
            "SETUP",
            "Unsupported server type: {} (supported: {:?})",
            config.server_type,
            server_config.supported_server_types
        );
        return Err(format!(
            "Unsupported server type: {} (supported: {:?})",
            config.server_type, server_config.supported_server_types
        ));
    }

    let repo = config
        .repository
        .as_ref()
        .ok_or("GitHub repository not specified")?;

    let server_dir = format!("{}/{}", server_config.mcp_servers_dir, server_name);
    log_debug!("SETUP", "Target directory: {}", server_dir);

    // Clone repository if not exists
    let mut need_install = false;
    if !tokio::fs::metadata(&server_dir).await.is_ok() {
        log_info!("SETUP", "Cloning {} to {}", repo, server_dir);

        let clone_start = Instant::now();
        let output = Command::new("git")
            .args(&[
                "clone",
                &format!("https://github.com/{}", repo),
                &server_dir,
            ])
            .output()
            .await
            .map_err(|e| {
                log_error!("SETUP", "Failed to execute git: {}", e);
                format!("Failed to execute git: {}", e)
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log_error!("SETUP", "Git clone failed: {}", stderr);
            return Err(format!("Git clone failed: {}", stderr));
        }

        log_info!(
            "SETUP",
            "Repository cloned successfully in {:?}",
            clone_start.elapsed()
        );
        need_install = true;
    } else {
        log_debug!(
            "SETUP",
            "Directory {} already exists, skipping clone",
            server_dir
        );
    }

    // Check if entrypoint exists, if not we need to run install command
    let entrypoint_path = format!("{}/{}", server_dir, config.entrypoint);
    if !tokio::fs::metadata(&entrypoint_path).await.is_ok() {
        log_warn!(
            "SETUP",
            "Entrypoint not found: {}, will run install command",
            entrypoint_path
        );
        need_install = true;
    }

    // Install dependencies if needed
    if need_install {
        if let Some(install_cmd) = &config.install_command {
            log_info!("SETUP", "Installing dependencies: {}", install_cmd);

            let install_start = Instant::now();

            // Handle complex commands with shell execution
            let output = if install_cmd.contains("&&") || install_cmd.contains("||") {
                // Use shell for complex commands
                Command::new("sh")
                    .args(&["-c", install_cmd])
                    .current_dir(&server_dir)
                    .output()
                    .await
                    .map_err(|e| {
                        log_error!(
                            "SETUP",
                            "Failed to execute install command via shell: {}",
                            e
                        );
                        format!("Failed to execute install command via shell: {}", e)
                    })?
            } else {
                // Use direct execution for simple commands
                let parts: Vec<&str> = install_cmd.split_whitespace().collect();
                if parts.is_empty() {
                    return Err("Empty install command".to_string());
                }
                Command::new(parts[0])
                    .args(&parts[1..])
                    .current_dir(&server_dir)
                    .output()
                    .await
                    .map_err(|e| {
                        log_error!("SETUP", "Failed to execute install command: {}", e);
                        format!("Failed to execute install command: {}", e)
                    })?
            };

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                log_error!("SETUP", "Install command failed: {}", stderr);
                log_error!("SETUP", "Install command stdout: {}", stdout);
                return Err(format!(
                    "Install command failed: {}\nstdout: {}",
                    stderr, stdout
                ));
            }

            log_info!(
                "SETUP",
                "Dependencies installed in {:?}",
                install_start.elapsed()
            );
        } else {
            log_warn!(
                "SETUP",
                "Entrypoint missing but no install command specified"
            );
        }
    }

    // Verify entrypoint exists after installation
    log_debug!("SETUP", "Final check - entrypoint: {}", entrypoint_path);

    if !tokio::fs::metadata(&entrypoint_path).await.is_ok() {
        log_error!("SETUP", "Entrypoint not found: {}", entrypoint_path);
        return Err(format!("Entrypoint not found: {}", entrypoint_path));
    }

    log_debug!("SETUP", "Entrypoint verified: {}", entrypoint_path);

    // Test Node.js if language is node
    if config.language == "node" {
        log_debug!("SETUP", "Testing Node.js installation");

        let node_test = Command::new("node")
            .args(&["--version"])
            .output()
            .await
            .map_err(|e| {
                log_error!("SETUP", "Node.js not found: {}", e);
                format!("Node.js not found: {}", e)
            })?;

        if !node_test.status.success() {
            log_error!("SETUP", "Node.js is not working");
            return Err("Node.js is not working".to_string());
        }

        let version = String::from_utf8_lossy(&node_test.stdout);
        log_info!("SETUP", "Node.js version: {}", version.trim());
    }

    log_info!("SETUP", "Server {} ready at {}", server_name, server_dir);
    Ok(())
}

fn build_command(
    server_name: &str,
    config: &McpServerConfig,
    server_config: &ServerConfig,
) -> Result<(String, Vec<String>), String> {
    let server_path = format!("{}/{}", server_config.mcp_servers_dir, server_name);
    let entrypoint_path = format!("{}/{}", server_path, config.entrypoint);

    if !server_config.supported_languages.contains(&config.language) {
        return Err(format!(
            "Unsupported language: {} (supported: {:?})",
            config.language, server_config.supported_languages
        ));
    }

    match config.language.as_str() {
        "node" => Ok(("node".to_string(), vec![entrypoint_path])),
        "python" => Ok(("python".to_string(), vec![entrypoint_path])),
        lang => Err(format!(
            "Language '{}' is supported but not implemented",
            lang
        )),
    }
}

async fn start_mcp_server(
    config_file: &str,
    server_name: &str,
    server_config: &ServerConfig,
) -> Result<McpServerProcess, Box<dyn std::error::Error + Send + Sync>> {
    log_info!(
        "MCP_SERVER",
        "Starting MCP server setup for '{}'",
        server_name
    );
    log_debug!("MCP_SERVER", "Loading config from: {}", config_file);
    log_debug!("MCP_SERVER", "Server config: {:?}", server_config);

    let config_content = tokio::fs::read_to_string(config_file)
        .await
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let configs: McpServersConfig = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    let mcp_server_config = configs
        .get(server_name)
        .ok_or_else(|| format!("Server '{}' not found in config", server_name))?;

    // Setup server
    setup_mcp_server(server_name, mcp_server_config, server_config)
        .await
        .map_err(|e| format!("Setup failed: {}", e))?;

    // Build command
    let (command, args) = build_command(server_name, mcp_server_config, server_config)
        .map_err(|e| format!("Command build failed: {}", e))?;

    log_info!("MCP_SERVER", "Starting process: {} {:?}", command, args);

    // Start process with enhanced logging
    let process_start = Instant::now();
    let mut child = Command::new(&command)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true) // Ensure cleanup
        .spawn()
        .map_err(|e| {
            log_error!("MCP_SERVER", "Failed to spawn process: {}", e);
            format!("Failed to spawn process: {}", e)
        })?;

    let process_id = child.id();
    log_info!("MCP_SERVER", "Process spawned with PID: {:?}", process_id);

    // Create child handle for process monitoring
    let child_handle = Arc::new(Mutex::new(Some(child)));
    let child_handle_clone = child_handle.clone();

    // Verify process is running
    tokio::time::sleep(Duration::from_millis(100)).await;

    let (stdin, stdout, stderr) = {
        let mut child_guard = child_handle.lock().await;
        if let Some(ref mut child) = child_guard.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    log_error!(
                        "MCP_SERVER",
                        "Process exited immediately with status: {}",
                        status
                    );
                    return Err(
                        format!("Process exited immediately with status: {}", status).into(),
                    );
                }
                Ok(None) => {
                    log_info!(
                        "MCP_SERVER",
                        "Process is running healthy - PID: {:?}, startup time: {:?}",
                        process_id,
                        process_start.elapsed()
                    );
                }
                Err(e) => {
                    log_error!("MCP_SERVER", "Failed to check process status: {}", e);
                    return Err(format!("Failed to check process status: {}", e).into());
                }
            }

            let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
            let stdout = child.stdout.take().ok_or("Failed to get stdout")?;
            let stderr = child.stderr.take().ok_or("Failed to get stderr")?;

            (stdin, stdout, stderr)
        } else {
            return Err("Failed to access child process".into());
        }
    };

    // Enhanced stderr monitoring with detailed logging
    let server_name_clone = server_name.to_string();
    let child_handle_monitor = child_handle.clone();
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        let mut line_count = 0;

        log_debug!(
            "STDERR_MONITOR",
            "Starting stderr monitoring for {}",
            server_name_clone
        );

        while let Ok(n) = reader.read_line(&mut line).await {
            if n == 0 {
                log_warn!(
                    "STDERR_MONITOR",
                    "Process {} terminated (stderr closed)",
                    server_name_clone
                );

                // Check actual process status when stderr closes
                let mut child_guard = child_handle_monitor.lock().await;
                if let Some(child) = child_guard.as_mut() {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            log_error!(
                                "STDERR_MONITOR",
                                "Process {} exited with status: {}",
                                server_name_clone,
                                status
                            );
                        }
                        Ok(None) => {
                            log_warn!(
                                "STDERR_MONITOR",
                                "Process {} stderr closed but process still running",
                                server_name_clone
                            );
                        }
                        Err(e) => {
                            log_error!("STDERR_MONITOR", "Failed to check process status: {}", e);
                        }
                    }
                }
                break;
            }

            line_count += 1;
            let trimmed_line = line.trim();

            if !trimmed_line.is_empty() {
                log_debug!(
                    "STDERR_MONITOR",
                    "[{}:{}] {}",
                    server_name_clone,
                    line_count,
                    trimmed_line
                );
            }

            line.clear();
        }

        log_info!(
            "STDERR_MONITOR",
            "Stderr monitoring ended for {} after {} lines",
            server_name_clone,
            line_count
        );
    });

    // Wait for process initialization with progress logging
    log_debug!(
        "MCP_SERVER",
        "Waiting for process initialization ({}s)",
        server_config.process_init_wait_secs
    );
    tokio::time::sleep(Duration::from_secs(server_config.process_init_wait_secs)).await;

    let now = Instant::now();
    log_info!(
        "MCP_SERVER",
        "MCP server '{}' started successfully - Total setup time: {:?}",
        server_name,
        process_start.elapsed()
    );

    Ok(McpServerProcess {
        stdin,
        stdout: BufReader::new(stdout),
        process_id,
        start_time: now,
        request_count: 0,
        last_activity: now,
        child_handle: child_handle_clone,
        config: server_config.clone(),
    })
}

// --- Authentication middleware ---
async fn auth_middleware(
    State(auth_config): State<AuthConfig>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    if !auth_config.enabled {
        return Ok(next.run(request).await);
    }

    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| &h[7..]);

    match (&auth_config.api_key, auth_header) {
        (Some(expected), Some(provided)) if expected == provided => Ok(next.run(request).await),
        _ => {
            let error = AuthError {
                error: "Unauthorized".to_string(),
                message: "Invalid or missing API key".to_string(),
            };
            Err((StatusCode::UNAUTHORIZED, AxumJson(error)))
        }
    }
}

// --- Request handler ---
async fn handle_request(
    State(mcp_process): State<Arc<Mutex<McpServerProcess>>>,
    AxumJson(payload): AxumJson<McpRequest>,
) -> Result<AxumJson<McpResponse>, StatusCode> {
    let request_start = Instant::now();
    log_info!("HTTP_HANDLER", "Received HTTP request");
    log_debug!("HTTP_HANDLER", "Request payload: {:?}", payload);

    // Acquire lock with timing
    let lock_start = Instant::now();
    let mut process = mcp_process.lock().await;
    log_debug!(
        "HTTP_HANDLER",
        "Acquired process lock in {:?}",
        lock_start.elapsed()
    );

    // Log process stats before query
    log_debug!("HTTP_HANDLER", "Process stats: {}", process.get_stats());

    match process.query(&payload).await {
        Ok(response) => {
            log_info!(
                "HTTP_HANDLER",
                "Request completed successfully in {:?}",
                request_start.elapsed()
            );
            log_debug!(
                "HTTP_HANDLER",
                "Response size: {} chars",
                response.result.len()
            );
            Ok(AxumJson(response))
        }
        Err(e) => {
            log_error!(
                "HTTP_HANDLER",
                "Request failed after {:?}: {}",
                request_start.elapsed(),
                e
            );
            log_error!(
                "HTTP_HANDLER",
                "Process stats at failure: {}",
                process.get_stats()
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// --- Main function ---
#[tokio::main]
async fn main() {
    let app_start = Instant::now();
    log_info!("MAIN", "Starting MCP HTTP server...");

    // Load server configuration from environment
    let server_config = ServerConfig::default();
    log_debug!("MAIN", "Server configuration: {:?}", server_config);

    // Configuration with detailed logging
    let api_key = env::var("HTTP_API_KEY").ok();
    let disable_auth = env::var("DISABLE_AUTH")
        .unwrap_or_default()
        .parse::<bool>()
        .unwrap_or(false);

    let auth_config = AuthConfig {
        enabled: !disable_auth && api_key.is_some(),
        api_key: api_key.clone(),
    };

    let config_file =
        env::var("MCP_CONFIG_FILE").unwrap_or_else(|_| DEFAULT_CONFIG_FILE.to_string());
    let server_name =
        env::var("MCP_SERVER_NAME").unwrap_or_else(|_| DEFAULT_SERVER_NAME.to_string());

    log_info!("MAIN", "Configuration loaded:");
    log_info!("MAIN", "  - Config file: {}", config_file);
    log_info!("MAIN", "  - Server name: {}", server_name);
    log_info!("MAIN", "  - Auth enabled: {}", auth_config.enabled);
    log_info!("MAIN", "  - API key present: {}", api_key.is_some());
    log_info!("MAIN", "  - Disable auth flag: {}", disable_auth);

    // Start MCP server with timing
    log_info!("MAIN", "Initializing MCP server...");
    let mcp_start = Instant::now();
    let mcp_process = match start_mcp_server(&config_file, &server_name, &server_config).await {
        Ok(process) => {
            log_info!(
                "MAIN",
                "MCP server initialized in {:?}",
                mcp_start.elapsed()
            );
            Arc::new(Mutex::new(process))
        }
        Err(e) => {
            log_error!(
                "MAIN",
                "Failed to start MCP server after {:?}: {}",
                mcp_start.elapsed(),
                e
            );
            return;
        }
    };

    // Setup HTTP server with enhanced logging
    log_info!("MAIN", "Setting up HTTP server...");
    let app = Router::new()
        .route("/api/v1", post(handle_request))
        .layer(middleware::from_fn_with_state(
            auth_config.clone(),
            auth_middleware,
        ))
        .with_state(mcp_process);

    let port = env::var("PORT").unwrap_or_else(|_| DEFAULT_PORT.to_string());
    let host = env::var("HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let addr = format!("{}:{}", host, port);

    log_info!("MAIN", "Attempting to bind to: {}", addr);

    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            let local_addr = listener.local_addr().unwrap();
            log_info!("MAIN", "Server ready at http://{}", local_addr);
            log_info!("MAIN", "Endpoint: POST /api/v1");
            log_info!("MAIN", "Total startup time: {:?}", app_start.elapsed());
            log_info!("MAIN", "Server is now accepting connections...");

            if let Err(e) = axum::serve(listener, app.into_make_service()).await {
                log_error!("MAIN", "Server error: {}", e);
            }
        }
        Err(e) => {
            log_error!("MAIN", "Failed to bind to {}: {}", addr, e);
        }
    }
}
