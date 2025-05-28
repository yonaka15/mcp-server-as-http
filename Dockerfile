# Multi-stage build for mcp-http-server
# Stage 1: Build stage with Rust only
FROM rust:1.85-slim-bookworm as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
  pkg-config \
  libssl-dev \
  curl \
  && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf src target/release/deps/mcp*

# Copy source code
COPY src ./src

# Build the application
RUN cargo build --release

# Stage 2: Runtime stage with Docker support
FROM debian:bookworm-slim

# Install runtime dependencies including Docker
RUN apt-get update && apt-get install -y \
  ca-certificates \
  curl \
  docker.io \
  && rm -rf /var/lib/apt/lists/*

# Create non-root user for security and add to docker group
RUN groupadd -r mcpuser && useradd -r -g mcpuser mcpuser && \
  usermod -aG docker mcpuser

# Set working directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/mcp-http-server .

# Copy configuration files and proxy script
COPY mcp_servers.config.json .
COPY .env.example .env
COPY github-mcp-proxy.sh .

# Make scripts executable
RUN chmod +x /app/mcp-http-server /app/github-mcp-proxy.sh && \
  chown -R root:root /app

# Note: Running as root to access Docker socket
# USER mcpuser

# Expose port
EXPOSE 3000

# Set environment variables with defaults
ENV MCP_CONFIG_FILE=mcp_servers.config.json
ENV MCP_SERVER_NAME=github
ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/api/v1 -X POST \
  -H "Content-Type: application/json" \
  -d '{"command":"test"}' || exit 1

# Run the application
CMD ["./mcp-http-server"]
