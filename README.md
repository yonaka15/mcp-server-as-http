# MCP HTTP Server - Redmine

Simple HTTP API for Redmine via Model Context Protocol.

## Quick Start

```bash
# Test
./test.sh

# Or manually
docker build -t mcp-http-server .
docker run -d --name mcp-http-server -p 1582:3000 --env-file .env mcp-http-server
```

## Configuration

### Install Command Options

The server supports multiple installation patterns:

1. **Recommended (with package-lock.json)**: `(npm ci || npm install) && npm run build`
2. **Alternative**: `npm install && npm run build`
3. **Development**: `npm run build` (if dependencies already installed)

**`mcp_servers.config.json`**:
```json
{
  "redmine": {
    "type": "github",
    "repository": "yonaka15/mcp-server-redmine",
    "language": "node",
    "entrypoint": "dist/index.js",
    "description": "MCP server for Redmine",
    "install_command": "(npm ci || npm install) && npm run build"
  }
}
```

**Environment** (`.env`):
```bash
PORT=1582
DISABLE_AUTH=true
MCP_SERVER_NAME=redmine
HTTP_API_KEY=your-secret-key

# Redmine Configuration
REDMINE_HOST=https://your-redmine.example.com
REDMINE_API_KEY=your-redmine-api-key-here
```

## Usage

```bash
curl -X POST http://localhost:1582/api/v1 \
  -H "Content-Type: application/json" \
  -d '{
    "command": "{
      \"jsonrpc\": \"2.0\", 
      \"id\": 1, 
      \"method\": \"tools/call\", 
      \"params\": {
        \"name\": \"search_issues\",
        \"arguments\": {\"project_id\": \"1\", \"limit\": 10}
      }
    }"
  }'
```

## Architecture

1. **Rust HTTP Server** - Handles REST API
2. **MCP Protocol** - Communicates with tools
3. **Redmine Server** - Manages issues and projects
4. **Auto Setup** - Clones and installs from GitHub

Clean, simple, fast.
