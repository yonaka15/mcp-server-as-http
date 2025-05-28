# GitHub MCP HTTP Server

A HTTP server that provides a REST API interface to GitHub's Model Context Protocol (MCP) server, enabling HTTP-based access to GitHub APIs.

## Features

- **GitHub API Access**: Full access to GitHub APIs via HTTP REST interface
- **REST API Interface**: Convert GitHub MCP protocol to HTTP REST API
- **Authentication**: Bearer token authentication support + GitHub Personal Access Token
- **Docker Support**: Single container deployment with GitHub MCP Server
- **Health Checks**: Built-in health checking capabilities
- **Logging**: Comprehensive debug logging

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

# Run the container
docker run -d \
  --name mcp-http-server \
  -p 3000:3000 \
  --env-file .env \
  mcp-http-server
```

## Configuration

### Environment Variables

Create a `.env` file (copy from `.env.example`):

```bash
# HTTP Server Authentication
HTTP_API_KEY=your-secret-api-key-here
DISABLE_AUTH=false

# MCP Server Configuration
MCP_CONFIG_FILE=mcp_servers.config.json
MCP_SERVER_NAME=github

# GitHub Personal Access Token
GITHUB_PERSONAL_ACCESS_TOKEN=your-github-token-here
```

### GitHub Personal Access Token

You need a GitHub Personal Access Token to use the GitHub MCP Server:

1. Go to [GitHub Settings > Personal Access Tokens](https://github.com/settings/personal-access-tokens/new)
2. Create a new token with the required permissions
3. Set the token in your `.env` file:

```bash
GITHUB_PERSONAL_ACCESS_TOKEN=your-github-token-here
```

### MCP Server Configuration

The `mcp_servers.config.json` is pre-configured for GitHub using Docker:

```json
{
  "github": {
    "command": "docker",
    "args": ["run", "-i", "--rm", "--network", "host", "-e", "GITHUB_PERSONAL_ACCESS_TOKEN", "ghcr.io/github/github-mcp-server"]
  }
}
```

## API Usage

### Authentication

Include Bearer token in Authorization header:

```bash
curl -X POST http://localhost:3000/api/v1 \
  -H "Authorization: Bearer your-secret-api-key-here" \
  -H "Content-Type: application/json" \
  -d '{"command": "your-mcp-command"}'
```

### Without Authentication

Set `DISABLE_AUTH=true` in your `.env` file:

```bash
curl -X POST http://localhost:3000/api/v1 \
  -H "Content-Type: application/json" \
  -d '{"command": "your-mcp-command"}'
```

## Development

### Local Development

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js dependencies globally (for MCP servers)
npm install -g @modelcontextprotocol/server-brave-search

# Build and run
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

- **Stage 1 (Builder)**: Rust + Node.js environment for building
- **Stage 2 (Runtime)**: Minimal Node.js runtime with compiled binary

### Security Features

- Non-root user execution
- Minimal runtime dependencies
- Optional Bearer token authentication
- Health check endpoints

### Dependencies

- **Runtime**: Docker (for GitHub MCP Server container)
- **Build**: Rust toolchain
- **GitHub Integration**: Official GitHub MCP Server Docker image

## Monitoring

### Health Check

```bash
# Check container health
docker ps

# Manual health check
curl -f http://localhost:3000/api/v1 \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"command":"test"}'
```

### Logs

```bash
# Docker logs
docker logs -f mcp-http-server

# Docker Compose logs
docker-compose logs -f
```

## Troubleshooting

### Common Issues

1. **Docker permission denied**: Ensure Docker socket is accessible (mounted in compose)
2. **GitHub authentication failure**: Check your `GITHUB_PERSONAL_ACCESS_TOKEN` is valid
3. **Permission denied**: Verify GitHub token has required permissions
4. **Port conflicts**: Change the port mapping in Docker commands
5. **GitHub MCP Server pull failed**: Ensure Docker can access ghcr.io

### Debug Mode

Enable debug logging:

```bash
# Set environment variable
export RUST_LOG=debug

# Or in .env file
RUST_LOG=debug
```

## License

This project is open source. Please refer to the LICENSE file for details.
