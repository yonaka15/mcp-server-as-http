# Multi-stage build for mcp-http-server
# Stage 1: Build stage with Rust and Node.js
FROM rust:1.85-slim-bookworm as builder

# Install Node.js and build dependencies
RUN apt-get update && apt-get install -y \
  pkg-config \
  libssl-dev \
  curl \
  && curl -fsSL https://deb.nodesource.com/setup_18.x | bash - \
  && apt-get install -y nodejs \
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

# Stage 2: Runtime stage
FROM node:20-slim

# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
  ca-certificates \
  curl \
  && rm -rf /var/lib/apt/lists/*

# Install MCP Filesystem Server globally before switching users
RUN npm install -g @modelcontextprotocol/server-filesystem

# Create non-root user for security
RUN groupadd -r mcpuser && useradd -r -g mcpuser mcpuser

# Set working directory
WORKDIR /app

# Create workspace directory and set permissions
RUN mkdir -p /workspace && \
    chown -R mcpuser:mcpuser /workspace

# Copy the binary from builder stage
COPY --from=builder /app/target/release/mcp-http-server .

# Copy configuration files
COPY mcp_servers.config.json .
COPY .env.example .env

# Set ownership of app directory
RUN chown -R mcpuser:mcpuser /app

# Switch to non-root user
USER mcpuser

# Expose port
EXPOSE 3000

# Set environment variables with defaults
# ENV MCP_CONFIG_FILE=mcp_servers.config.json
# ENV MCP_SERVER_NAME=brave-search
# ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/api/v1 -X POST \
  -H "Content-Type: application/json" \
  -d '{"command":"test"}' || exit 1

# Run the application
CMD ["./mcp-http-server"]
