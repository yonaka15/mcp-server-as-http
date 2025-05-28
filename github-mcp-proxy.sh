#!/bin/bash

# GitHub MCP Server Proxy Script
# This script runs the GitHub MCP Server Docker container and handles communication

set -e

# Check if GitHub token is set
if [ -z "$GITHUB_PERSONAL_ACCESS_TOKEN" ]; then
    echo "Error: GITHUB_PERSONAL_ACCESS_TOKEN environment variable is not set" >&2
    exit 1
fi

# Pull the latest GitHub MCP Server image if not exists
if ! docker image inspect ghcr.io/github/github-mcp-server >/dev/null 2>&1; then
    echo "Pulling GitHub MCP Server Docker image..." >&2
    docker pull ghcr.io/github/github-mcp-server >/dev/null 2>&1
fi

# Start the GitHub MCP Server container (default mode is stdio)
exec docker run -i --rm \
    -e GITHUB_PERSONAL_ACCESS_TOKEN="$GITHUB_PERSONAL_ACCESS_TOKEN" \
    ghcr.io/github/github-mcp-server
