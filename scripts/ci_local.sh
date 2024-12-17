#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "🔍 Running local checks..."

# Check if Scarb is installed
if ! command -v scarb &> /dev/null; then
    echo -e "${RED}❌ Scarb is not installed. Please install it first.${NC}"
    exit 1
fi

# Check if Python is installed
if ! command -v python3 &> /dev/null; then
    echo -e "${RED}❌ Python 3 is not installed. Please install it first.${NC}"
    exit 1
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Rust is not installed. Please install it first.${NC}"
    exit 1
fi

echo "📦 Checking Rust formatting..."
cd client-rs || exit 1
cargo fmt --all -- --check
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Rust formatting check failed${NC}"
    exit 1
fi

echo "🔍 Running Clippy..."
cargo clippy -- -D warnings
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Clippy check failed${NC}"
    exit 1
fi
cd ..

# Setup if not already done
if [ ! -d "venv" ]; then
    echo "🔧 Setting up project..."
    make setup
fi

# Activate virtual environment
source venv/bin/activate

echo "📝 Checking Python formatting..."
./scripts/check_format.sh
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Python formatting check failed${NC}"
    exit 1
fi

echo "🧪 Running Cairo-Zero tests..."
make test
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Tests failed${NC}"
    exit 1
fi

echo -e "${GREEN}✅ All checks passed successfully!${NC}" 