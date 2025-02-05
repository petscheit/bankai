# Use a newer Rust image as the base
FROM rust:1.82-slim as builder

# Create a new empty shell project
WORKDIR /usr/src/app

# Install OpenSSL development packages and pkg-config
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first (better layer caching)
COPY client-rs/Cargo.toml client-rs/Cargo.lock client-rs/

# Create a dummy main.rs to build dependencies
RUN mkdir -p client-rs/src && \
    echo 'fn main() { println!("Dummy"); }' > client-rs/src/main.rs && \
    # Try to build dependencies
    cd client-rs && cargo build --release || true && \
    rm -rf src/

# Now copy the real source code
COPY client-rs/src client-rs/src/
COPY client-rs/migrations client-rs/migrations/
COPY client-rs/scripts client-rs/scripts/

# Build your application
WORKDIR /usr/src/app/client-rs
RUN cargo build --release --bin daemon

# Start with a regular Debian image for the runtime (not slim)
FROM debian:bookworm

# Install OpenSSL dependencies and PostgreSQL client
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    postgresql-client-15 \
    postgresql-15 \
    && rm -rf /var/lib/apt/lists/* \
    && which psql

# Create required directories
RUN mkdir -p /batches \
    /usr/src/app/cairo/build \
    /usr/src/app/contract/target/release \
    /usr/src/app/migrations \
    /usr/src/app/scripts

# Copy the build artifact from the builder stage
COPY --from=builder /usr/src/app/client-rs/target/release/daemon /usr/local/bin/daemon

# Copy scripts from builder
COPY --from=builder /usr/src/app/client-rs/scripts /usr/src/app/client-rs/scripts/

# Make the scripts executable
RUN chmod +x /usr/src/app/client-rs/scripts/run-migrations.sh \
    && chmod +x /usr/src/app/client-rs/scripts/entrypoint.sh

# Copy migrations from builder
COPY --from=builder /usr/src/app/client-rs/migrations /usr/src/app/client-rs/migrations/

# Make the scripts executable (after copying them)
RUN chmod +x /usr/src/app/client-rs/scripts/run-migrations.sh \
    && chmod +x /usr/src/app/client-rs/scripts/entrypoint.sh

# Copy Cairo build files
COPY cairo/build/epoch_batch.json /usr/src/app/cairo/build/epoch_batch.json
COPY cairo/build/committee_update.json /usr/src/app/cairo/build/committee_update.json

# Copy contract class file
COPY contract/target/release/bankai_BankaiContract.contract_class.json /usr/src/app/contract/target/release/bankai_BankaiContract.contract_class.json

# Use ENTRYPOINT with CMD for better control
ENTRYPOINT ["/usr/src/app/client-rs/scripts/entrypoint.sh"]
CMD []
