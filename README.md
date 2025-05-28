# MCP HTTP Server

A HTTP server that provides a REST API interface to Model Context Protocol (MCP) servers.

## Features

- **REST API Interface**: Convert MCP protocol to HTTP REST API
- **Authentication**: Bearer token authentication support
- **Configuration**: JSON-based MCP server configuration
- **Docker Support**: Full Docker containerization with multi-stage builds
- **GitHub MCP Server Integration**: Supports using `github/github-mcp-server` for GitHub API interactions.
- **Health Checks**: Built-in health checking capabilities
- **Logging**: Configurable logging via `RUST_LOG`.

## Quick Start with Docker

### 1. Build and Run (Recommended)

```bash
# Make the build script executable
chmod +x docker-build.sh

# Build and start the container
./docker-build.sh

# Or with custom options
./docker-build.sh --port 8080 --tag v1.0.0
```

### 2. Using Docker Compose

```bash
# Start the service
docker-compose up -d

# View logs
docker-compose logs -f

# Stop the service
docker-compose down
```

### 3. Manual Docker Commands

```bash
# Build the image
docker build -t mcp-http-server .

# Run the container (ensure .env file is configured or pass env vars directly)
docker run -d \
  --name mcp-http-server \
  -p 3000:3000 \
  --env-file .env \
  mcp-http-server
```

## Configuration

### Environment Variables

Create a `.env` file (copy from `.env.example`) and customize it:

```bash
# MCP HTTP Server Configuration

# HTTP Server Authentication
# Set your API key here to enable Bearer token authentication.
# If HTTP_API_KEY is set and DISABLE_AUTH is false, Authorization header is required.
HTTP_API_KEY=your-secret-api-key-here

# Set to 'true' to disable authentication completely.
DISABLE_AUTH=false

# MCP Server Configuration
# Path to the MCP servers configuration file.
MCP_CONFIG_FILE=mcp_servers.config.json

# Key of the MCP server to use from MCP_CONFIG_FILE.
# Examples: "brave-search", "github"
MCP_SERVER_NAME=github # Set to "github" to use the GitHub MCP Server

# Logging Configuration (optional)
# Supported levels: error, warn, info, debug, trace. Defaults to "info".
RUST_LOG=info

# --- GitHub MCP Server Specific Configuration ---
# REQUIRED if MCP_SERVER_NAME=github
# Your GitHub Personal Access Token.
# This will be passed to the github-mcp-server process.
# Example: GITHUB_PERSONAL_ACCESS_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
GITHUB_PERSONAL_ACCESS_TOKEN="your_actual_github_pat_here"

# Optional: Comma-separated list of toolsets for github-mcp-server.
# If set in this .env file, this will override the GITHUB_TOOLSETS value 
# in mcp_servers.config.json for the 'github' server.
# Available: repos, issues, users, pull_requests, code_security, experiments, all
# Example: GITHUB_TOOLSETS="repos,issues,pull_requests"
# GITHUB_TOOLSETS="repos,issues,pull_requests,users,code_security" 
```
**Important:**
- The `GITHUB_PERSONAL_ACCESS_TOKEN` environment variable in your `.env` file will be used by the `mcp-http-server`.
- This token is then passed to the `github-mcp-server` process if its configuration in `mcp_servers.config.json` includes `GITHUB_PERSONAL_ACCESS_TOKEN` in its `env` map (which it does by default).
- Ensure the token has the necessary permissions on GitHub for the operations you intend to perform.

### MCP Server Configuration

Edit `mcp_servers.config.json` to configure MCP servers. The `github-mcp-server` is now included by default:

```json
{
  "brave-search": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-brave-search"]
  },
  "github": {
    "command": "/usr/local/bin/github-mcp-server",
    "args": ["stdio"],
    "env": {
      "GITHUB_PERSONAL_ACCESS_TOKEN": "CONFIGURE_YOUR_GITHUB_PAT_IN_.ENV_OR_DOCKER_RUN",
      "GITHUB_TOOLSETS": "repos,issues,pull_requests,users,code_security"
    }
  }
}
```
- The `command` for `"github"` points to the `github-mcp-server` binary inside the Docker container.
- The `env.GITHUB_PERSONAL_ACCESS_TOKEN` value in this JSON file acts as a placeholder if the `GITHUB_PERSONAL_ACCESS_TOKEN` environment variable is not set when running `mcp-http-server`. It's highly recommended to set the actual token via the `.env` file or Docker environment variables for security.
- `env.GITHUB_TOOLSETS` specifies the default toolsets for the GitHub server. This can also be overridden by setting the `GITHUB_TOOLSETS` environment variable in your `.env` file (this override behavior was added to the Rust application).

## API Usage

To make a request to the configured MCP server, send a POST request to `/api/v1`.

### Authentication

If authentication is enabled (i.e., `DISABLE_AUTH=false` and `HTTP_API_KEY` is set in your `.env` file), include a Bearer token in the Authorization header:

```bash
curl -X POST http://localhost:3000/api/v1 \
  -H "Authorization: Bearer your-secret-api-key-here" \
  -H "Content-Type: application/json" \
  -d '{"command": "your-mcp-command-json-string"}'
```

### Without Authentication

If authentication is disabled (`DISABLE_AUTH=true` in your `.env` file):

```bash
curl -X POST http://localhost:3000/api/v1 \
  -H "Content-Type: application/json" \
  -d '{"command": "your-mcp-command-json-string"}'
```

### Example: Using the GitHub MCP Server

To use the GitHub MCP server, ensure `MCP_SERVER_NAME=github` is set in your `.env` file and `GITHUB_PERSONAL_ACCESS_TOKEN` is correctly configured.

The `command` field in the JSON payload should be a JSON string that `github-mcp-server` understands. This typically involves specifying a `tool_name` and `tool_input`.

Example: List repository branches for `octocat/Spoon-Knife`:

```bash
# Ensure your HTTP_API_KEY and GITHUB_PERSONAL_ACCESS_TOKEN are set in .env
# Ensure MCP_SERVER_NAME=github in .env

curl -X POST http://localhost:3000/api/v1 \
  -H "Authorization: Bearer your-secret-api-key-here" \
  -H "Content-Type: application/json" \
  -d '{ 
       "command": "{\"tool_name\":\"list_branches\",\"tool_input\":{\"owner\":\"octocat\",\"repo\":\"Spoon-Knife\"}}" 
     }'
```

Expected (partial) response if successful:
```json
{
  "result": "{\"tool_name\":\"list_branches\",\"transaction_id\":\"...",\"output\":[{\"name\":\"main\",\"commit\":{\"sha\":\"...",\"url\":\"..."}}]}"
}
```

(Note: The exact command format and response structure for `github-mcp-server` should be referred from its own documentation. The above is an illustrative example.)

## Development

### Local Development

For local development of `mcp-http-server` (without Docker):

1.  **Install Rust**: If not already installed, get it from [rustup.rs](https://rustup.rs/).
2.  **Install Node.js**: Required for `npx` if you plan to test with MCP servers like `server-brave-search`.
3.  **(Optional) Install `github-mcp-server` binary**: If you want to test the GitHub integration locally without Docker, you would need to build or download the `github-mcp-server` binary and adjust the `command` path in `mcp_servers.config.json` accordingly.
4.  **Configure `.env`**: Copy `.env.example` to `.env` and set your variables (e.g., `MCP_SERVER_NAME`, `GITHUB_PERSONAL_ACCESS_TOKEN`).
5.  **Build and Run**:
    ```bash
    cargo build --release
    ./target/release/mcp-http-server
    ```

### Docker Development

```bash
# Build only (no run)
./docker-build.sh --build-only

# Build without cache
./docker-build.sh --no-cache

# Custom port
./docker-build.sh --port 8080
```

## Architecture

### Multi-stage Docker Build

The `Dockerfile` uses a multi-stage build approach:
- **Stage 0 (GitHub MCP Builder)**: Clones the `github/github-mcp-server` repository and builds its Go binary. This ensures the latest version is used at build time.
- **Stage 1 (Rust Builder)**: Uses a Rust base image with Node.js installed to build the `mcp-http-server` Rust binary.
- **Stage 2 (Runtime)**: Uses a minimal Node.js slim image as a base. It copies the compiled `mcp-http-server` binary from the `rust_builder` stage and the `github-mcp-server` binary from the `github_mcp_builder` stage. This stage includes Node.js to maintain compatibility for MCP servers that might be invoked via `npx` (like the default `brave-search` example).

### Security Features

- Non-root user execution in Docker.
- Minimal runtime dependencies in the final Docker image.
- Optional Bearer token authentication for the HTTP API.
- Health check endpoint.

### Dependencies

- **Runtime**:
  - `mcp-http-server` (Rust binary, this project)
  - `github-mcp-server` (Go binary, for GitHub integration, included in Docker image)
  - Node.js (included in Docker image for `npx` compatibility with other potential MCP servers)
  - `ca-certificates`, `curl` (in Docker image for general connectivity and health checks)
- **Build (for Docker image creation)**:
  - Rust toolchain
  - Go toolchain
  - Git
  - Node.js & npm (for Rust builder stage if any build scripts required it, and for npx usage)
- **MCP Servers (examples)**:
  - `@modelcontextprotocol/server-brave-search` (npm package, installed dynamically by `npx` if `brave-search` server is used)
  - `github-mcp-server` (Go binary, directly executed, included)

## Monitoring

### Health Check

The Docker container has a health check. To check its status:
```bash
docker ps
```
Look for the status in the `STATUS` column for the `mcp-http-server` container.

To manually invoke a similar check (if auth is disabled or `MCP_SERVER_NAME` points to a server that responds to `test` command simply):
```bash
# If DISABLE_AUTH=true in .env
curl -f http://localhost:3000/api/v1 \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"command":"test"}'

# If auth is enabled, include your API key
curl -f http://localhost:3000/api/v1 \
  -X POST \
  -H "Authorization: Bearer your-secret-api-key-here" \
  -H "Content-Type: application/json" \
  -d '{"command":"test"}'
```
Note: The `test` command's behavior depends on the selected MCP server.

### Logs

Set the `RUST_LOG` environment variable to control log verbosity (e.g., `info`, `debug`).

```bash
# Docker logs
docker logs -f mcp-http-server

# Docker Compose logs
docker-compose logs -f mcp-http-server # Or your service name if different
```

## Troubleshooting

### Common Issues

1.  **`github-mcp-server` fails to start**: 
    *   Ensure `GITHUB_PERSONAL_ACCESS_TOKEN` is correctly set in your `.env` file and has the required permissions on GitHub.
    *   Check logs for messages from `github-mcp-server` itself (they will be prefixed with `[MCP Server stderr - github]:`).
2.  **Node.js/npx not found (for `brave-search` or similar)**: This should not happen with the provided Dockerfile as Node.js is included in the runtime image.
3.  **MCP server startup failure (general)**: 
    *   Check network connectivity if an MCP server needs to download resources (like `server-brave-search`).
    *   Verify `command` and `args` in `mcp_servers.config.json` are correct for the chosen `MCP_SERVER_NAME`.
4.  **Permission denied**: File permissions within the Docker container are managed by the Dockerfile. If running locally, ensure the `mcp-http-server` binary and any MCP server binaries have execute permissions.
5.  **Port conflicts**: Change the host port mapping in Docker commands (e.g., `-p <new_host_port>:3000`).

### Debug Mode

Enable more verbose logging by setting the `RUST_LOG` environment variable:

```bash
# In your .env file or as an environment variable for Docker
RUST_LOG=debug
```
This will show more detailed logs from `mcp-http-server` and the MCP server interaction.

## License

This project is open source. Please refer to the LICENSE file for details.
