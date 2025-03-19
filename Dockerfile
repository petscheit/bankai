# Declare the Python version build argument globally.
ARG PYTHON_VERSION=3.10-slim

# Stage 1: Build the daemon binary with Rust
FROM python:${PYTHON_VERSION} AS builder

# Install required dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl3 \
    libssl-dev \
    curl \
    build-essential \
    && ln -s /usr/bin/python3 /usr/bin/python

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Optionally set the environment variable so that pyo3 uses python3.
ENV PYO3_PYTHON=python3

WORKDIR /app

# Copy manifest files to cache dependencies.
COPY Cargo.toml Cargo.lock ./

# Copy the rest of the source code.
COPY . .

# Build your daemon binary (adjust the package name if needed).
RUN cargo build --release -p bankai-daemon

# Stage 2: Final image with Python installed
FROM python:${PYTHON_VERSION}
WORKDIR /app

# Install the required runtime libraries, git, and build tools
RUN apt-get update && apt-get install -y \
pkg-config \
libssl3 \
libssl-dev \
git \
build-essential \
gcc \
libgmp-dev \
&& ln -s /usr/bin/python3 /usr/bin/python


# Copy the entire repository including .git
COPY . .

# Initialize git submodule
RUN git submodule update --init

# Create build directory
RUN mkdir -p cairo/build

# Install Python dependencies
RUN pip install --no-cache-dir --force-reinstall uv
RUN uv pip compile cairo/packages/garaga_zero/pyproject.toml --output-file requirements.txt -q
RUN uv pip install --no-cache-dir --force-reinstall --system -r requirements.txt
RUN pip install --no-cache-dir --force-reinstall py_ecc

# Set the environment variable for pyo3
ENV PYO3_PYTHON=python3
ENV PYTHONPATH=/usr/local/lib/python3.10:/app/cairo/packages/garaga_zero:/usr/local/lib/python3.10/site-packages

# Copy the compiled daemon binary from the builder stage
COPY --from=builder /app/target/release/bankai-daemon /app/bin/bankai-daemon

# Copy the compiled cairo files into the image. Adjust source/destination as needed.
COPY --from=builder /app/cairo/build/ /app/cairo/build/
COPY --from=builder /app/cairo/verifier/ /app/cairo/verifier/

# Define the entrypoint for the container.
ENTRYPOINT ["/app/bin/bankai-daemon"] 