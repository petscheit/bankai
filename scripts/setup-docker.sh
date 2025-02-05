#!/bin/bash

set -e  # Exit on error

# Set working directory
cd /usr/src/app

# Install Rust toolchain
echo "Installing Rust toolchain..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Debug: Check if .gitmodules exists
echo "Checking .gitmodules file:"
cat .gitmodules || echo ".gitmodules file not found!"


# Check if submodule directory exists and print submodule info
ls -la cairo/packages 2>/dev/null || echo "Warning: cairo/packages directory not found"

# Debug: Print contents of garaga_zero directory
echo "Contents of garaga_zero directory:"
ls -la cairo/packages/garaga_zero/ || echo "Error: Cannot access garaga_zero directory"

# Use uv to compile and install dependencies
uv pip compile cairo/packages/garaga_zero/pyproject.toml \
    --output-file cairo/packages/garaga_zero/tools/make/requirements.txt

# Install dependencies using --system flag
uv pip install --system -r cairo/packages/garaga_zero/tools/make/requirements.txt
uv pip install --system py_ecc

# Set up Python path
export PYTHONPATH="$PWD:$PWD/cairo/packages/garaga_zero:$PYTHONPATH"
SITE_PACKAGES=$(python3.10 -c "import site; print(site.getsitepackages()[0])")

# Ensure directories exist
mkdir -p $SITE_PACKAGES/starkware/cairo/lang/
mkdir -p $SITE_PACKAGES/starkware/

# Apply patches with more verbose output
echo "Applying patch to instances.py..."
patch -p1 -F 3 --verbose $SITE_PACKAGES/starkware/cairo/lang/instances.py < cairo/packages/garaga_zero/tools/make/instances.patch || {
    echo "Patch failed for instances.py"
    cat $SITE_PACKAGES/starkware/cairo/lang/instances.py.rej || true
}

echo "Applying patch to extension_field_modulo_circuit.py..."
patch $SITE_PACKAGES/garaga/extension_field_modulo_circuit.py < cairo/packages/garaga_zero/tools/make/extension_field_modulo_circuit.patch

# Clone and install forked garaga
git clone -b backup https://github.com/petscheit/garaga.git
cd garaga && git checkout backup && cd ..
uv pip install --system -e garaga