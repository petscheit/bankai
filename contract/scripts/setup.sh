# Check if Rust is installed
if ! command -v rustc &> /dev/null; then
    echo "Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source "$HOME/.cargo/env"
else
    echo "Rust is already installed"
fi

# Check if asdf is installed
if ! command -v asdf &> /dev/null; then
    echo "asdf is not installed. Please install asdf first."
    exit 1
fi

# Check and install scarb
if ! asdf list scarb &> /dev/null; then
    echo "Adding scarb plugin to asdf..."
    asdf plugin add scarb
    asdf install scarb latest
    asdf global scarb latest
else
    echo "scarb is already installed via asdf"
fi

# Check and install starknet-devnet
if ! asdf list starknet-devnet &> /dev/null; then
    echo "Adding starknet-devnet plugin to asdf..."
    asdf plugin add starknet-devnet
    asdf install starknet-devnet latest
    asdf global starknet-devnet latest
else
    echo "starknet-devnet is already installed via asdf"
fi

# Install starkli
cargo install --locked --git https://github.com/xJonathanLEI/starkli

echo "Setting up starknet-devnet Account..."
if [ ! -f "account.json" ]; then
    # Start starknet-devnet in the background with a fixed seed
    echo "Starting starknet-devnet with a fixed seed..."
    starknet-devnet --host 127.0.0.1 --port 5050 --seed 1337 &
    DEVNET_PID=$!

    # Wait a few seconds for devnet to start
    sleep 3

    # Create account file only if it doesn't exist
    echo "Creating account file..."
    starkli account fetch 0x046d40ee9ddf64f6a92b04f26902f67a76c93692b8637afd43daeeeebc836609 --output account.json --rpc http://127.0.0.1:5050
    
    trap "kill $DEVNET_PID" EXIT
else
    echo "Account file already exists"
fi

