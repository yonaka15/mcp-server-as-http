#!/bin/bash
# Simple test

echo "=== MCP HTTP Server Test ==="

# Cleanup and build
docker stop mcp-http-server 2>/dev/null || true
docker rm mcp-http-server 2>/dev/null || true

echo "Building..."
docker build -t mcp-http-server . || exit 1

echo "Starting..."
docker run -d --name mcp-http-server -p 1582:3000 --env-file .env mcp-http-server

echo "Waiting..."
sleep 15

echo "Testing..."
RESPONSE=$(curl -s -w "HTTP %{http_code}" -o /tmp/api_response.txt \
  -X POST http://localhost:1582/api/v1 \
  -H "Content-Type: application/json" \
  -d '{"command":"test"}')

echo "$RESPONSE"
echo "Response body:"
cat /tmp/api_response.txt
echo

echo "Recent logs:"
docker logs mcp-http-server --tail 20

echo "Done! Server at http://localhost:1582"
