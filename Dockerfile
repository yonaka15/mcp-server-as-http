# Multi-stage build for mcp-http-server
FROM rust:1.85-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
  pkg-config \
  libssl-dev \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Build Rust application
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src target/release/deps/mcp*

COPY src ./src
RUN cargo build --release

# Runtime stage
FROM node:18-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
  ca-certificates \
  curl \
  git \
  && rm -rf /var/lib/apt/lists/*

# Create user with proper home directory first
RUN groupadd -r mcpuser && useradd -r -g mcpuser -m -d /home/mcpuser mcpuser

# Set up working directory
WORKDIR /app

# Copy binary and configuration
COPY --from=builder /app/target/release/mcp-http-server .
COPY mcp_servers.config.json .
COPY .env.example .env

# Create MCP servers directory
RUN mkdir -p mcp-servers

# Set proper ownership for all directories
RUN chown -R mcpuser:mcpuser /app /home/mcpuser
RUN chmod -R 755 /home/mcpuser

# Switch to non-root user
USER mcpuser

EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/api/v1 -X POST \
  -H "Content-Type: application/json" \
  -d '{"command":"test"}' || exit 1

CMD ["./mcp-http-server"]
