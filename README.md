## Recent Updates (2025-06-09)

**🔧 Fixed Broken Pipe Error Issues:**
- Updated MCP server configuration to use direct `node` command instead of `npx`
- Enhanced error handling for Broken Pipe errors (errno 32)
- Upgraded to Node.js 20 for better stability
- Pre-installed MCP Filesystem Server in Docker image
- Improved Docker filesystem permissions

**📁 Filesystem Server Configuration:**
```json
{
  "filesystem": {
    "command": "node",
    "args": ["/usr/local/lib/node_modules/@modelcontextprotocol/server-filesystem/dist/index.js", "/workspace"]
  }
}
```

---

# MCP HTTP Server

A HTTP server that provides a REST API interface to Model Context Protocol (MCP) servers.

## Features

- **REST API Interface**: Convert MCP protocol to HTTP REST API
- **Authentication**: Bearer token authentication support
- **Configuration**: JSON-based MCP server configuration
- **Docker Support**: Full Docker containerization with multi-stage builds
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
MCP_SERVER_KEY=brave-search
```

### MCP Server Configuration

Edit `mcp_servers.config.json` to configure MCP servers:

```json
{
  "brave-search": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-brave-search"]
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

Example for `tools/list`:
```bash
 curl -X POST http://localhost:3000/api/v1 \
  -H "Authorization: Bearer your-secret-api-key-here" \
  -H "Content-Type: application/json" \
  -d '{"command": "{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"tools/list\", \"params\": {}}"}'
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

- **Runtime**: Node.js (for npx and MCP servers)
- **Build**: Rust toolchain
- **MCP Servers**: Various npm packages (installed dynamically)

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

1. **Node.js/npx not found**: Ensure Node.js is installed in the container
2. **MCP server startup failure**: Check network connectivity for npm package downloads
3. **Permission denied**: Verify file permissions and user configuration
4. **Port conflicts**: Change the port mapping in Docker commands

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
