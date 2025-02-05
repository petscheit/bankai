# Use a newer Rust image as the base
FROM rust:1.82-slim as builder

# Create working directory
WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Rust project files
COPY client-rs/Cargo.toml client-rs/Cargo.lock client-rs/

# Create dummy main.rs for dependency caching
RUN mkdir -p client-rs/src && \
    echo 'fn main() { println!("Dummy"); }' > client-rs/src/main.rs && \
    cd client-rs && cargo build --release || true && \
    rm -rf src/

# Copy actual source code
COPY client-rs/src client-rs/src/
COPY client-rs/migrations client-rs/migrations/
COPY client-rs/scripts client-rs/scripts/

# Build Rust application
WORKDIR /usr/src/app/client-rs
RUN cargo build --release --bin daemon

# Start fresh with Python image for runtime
FROM python:3.10-slim-bookworm

# Install runtime dependencies and build tools
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    postgresql-client-15 \
    postgresql-15 \
    git \
    curl \
    gcc \
    build-essential \
    libgmp-dev \
    && rm -rf /var/lib/apt/lists/* \
    && which psql

# Install uv from official image
COPY --from=ghcr.io/astral-sh/uv:0.5.28 /uv /uvx /bin/

# Create required directories
RUN mkdir -p /batches \
    /usr/src/app/cairo/build \
    /usr/src/app/contract/target/release \
    /usr/src/app/migrations \
    /usr/src/app/scripts

# Copy the Rust binary from builder
COPY --from=builder /usr/src/app/client-rs/target/release/daemon /usr/local/bin/daemon

# Copy scripts and migrations
COPY --from=builder /usr/src/app/client-rs/scripts /usr/src/app/client-rs/scripts/
COPY --from=builder /usr/src/app/client-rs/migrations /usr/src/app/client-rs/migrations/

# Make scripts executable
RUN chmod +x /usr/src/app/client-rs/scripts/run-migrations.sh \
    && chmod +x /usr/src/app/client-rs/scripts/entrypoint.sh

# Copy Cairo and contract files
COPY cairo/build/epoch_batch.json /usr/src/app/cairo/build/
COPY cairo/build/committee_update.json /usr/src/app/cairo/build/
COPY contract/target/release/bankai_BankaiContract.contract_class.json /usr/src/app/contract/target/release/
COPY cairo/py/ /usr/src/app/cairo/py/
COPY cairo/src /usr/src/app/cairo/src/

# Copy Python-related files
COPY .gitmodules /usr/src/app/

# Create base directory
RUN mkdir -p /usr/src/app/cairo/packages/garaga_zero

# Copy the minimal set of files needed for pip compile
COPY cairo/packages/garaga_zero/pyproject.toml /usr/src/app/cairo/packages/garaga_zero/

# Copy submodule files
COPY cairo/packages/garaga_zero/src/ /usr/src/app/cairo/packages/garaga_zero/src/
COPY cairo/packages/garaga_zero/precompiled_circuits/ /usr/src/app/cairo/packages/garaga_zero/precompiled_circuits/
COPY cairo/packages/garaga_zero/tools/ /usr/src/app/cairo/packages/garaga_zero/tools/

# Create tools/make directory
RUN mkdir -p /usr/src/app/cairo/packages/garaga_zero/tools/make

# Copy and execute setup script
COPY scripts/setup-docker.sh /usr/src/app/scripts/
RUN chmod +x /usr/src/app/scripts/setup-docker.sh
RUN /usr/src/app/scripts/setup-docker.sh

# Set Python path
ENV PYTHONPATH="/usr/src/app:/usr/src/app/cairo/packages/garaga_zero:${PYTHONPATH}"

# Use ENTRYPOINT with CMD
ENTRYPOINT ["/usr/src/app/client-rs/scripts/entrypoint.sh"]
CMD []
