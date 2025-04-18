FROM rust:1.74-slim

WORKDIR /usr/src/app

# Install necessary dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev curl && \
    rm -rf /var/lib/apt/lists/*

# Copy our code
COPY Cargo.toml ./
COPY src ./src

# Build the application
RUN cargo build --release

# Set up entrypoint script
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]