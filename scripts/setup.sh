# Check if python3.10 is installed
if ! command -v python3.10 >/dev/null; then
    echo "python3.10 is not installed. Please install Python 3.10 and try again."
    case "$OSTYPE" in
        linux-gnu*)
            echo "On Debian/Ubuntu, you can install it with: sudo apt-get install python3.10"
            echo "On Fedora, you can install it with: sudo dnf install python3.10"
            ;;
        darwin*)
            echo "On macOS, you can install it with Homebrew: brew install python@3.10"
            ;;
        *)
            echo "Please refer to your operating system's documentation for installing Python 3.10."
            ;;
    esac
    exit 1
fi

# Check if venv module is available
if ! python3.10 -m venv --help >/dev/null 2>&1; then
    echo "The venv module is not available in your Python 3.10 installation."
    case "$OSTYPE" in
        linux-gnu*)
            echo "On Debian/Ubuntu, you can install it with: sudo apt-get install python3.10-venv"
            echo "On Fedora, you can install it with: sudo dnf install python3.10-venv"
            ;;
        darwin*)
            echo "On macOS, ensure your Python 3.10 installation includes the venv module."
            ;;
        *)
            echo "Please refer to your operating system's documentation for installing the venv module."
            ;;
    esac
    exit 1
fi

echo "Fetching Garaga-zero as submodule..."
git submodule update --init
mkdir -p cairo/build


# Create virtual environment
if ! python3.10 -m venv venv; then
    echo "Failed to create virtual environment with python3.10"
    exit 1
fi

# Install Cairo tools before activating venv
OS_TYPE=$(uname -s)
echo "Detected OS: $OS_TYPE"  # Debug line
echo "Installing Cairo tools..."
case "$OS_TYPE" in
    Linux*)
        curl -L https://github.com/starkware-libs/cairo/releases/download/v2.4.0/cairo-2.4.0-x86_64-unknown-linux-musl.tar.gz -o cairo.tar.gz
        tar -xzf cairo.tar.gz
        sudo mv cairo/bin/* /usr/local/bin/
        rm -rf cairo.tar.gz cairo
        ;;
    Darwin*)
        if command -v brew >/dev/null; then
            brew install cairo
        else
            echo "Homebrew is not installed. Please install Homebrew and try again."
            exit 1
        fi
        ;;
    *)
        echo "Unsupported operating system for automatic Cairo installation."
        echo "OS_TYPE=$OS_TYPE"  # Debug line
        exit 1
        ;;
esac

# Function to install GNU parallel
install_parallel() {
    echo "Installing GNU parallel..."
    case "$OSTYPE" in
        linux-gnu*)
            # Linux
            if command -v apt-get >/dev/null; then
                sudo apt-get update && sudo apt-get install -y parallel
            elif command -v dnf >/dev/null; then
                sudo dnf install -y parallel
            else
                echo "Unsupported Linux distribution for automatic parallel installation."
                exit 1
            fi
            ;;
        darwin*)
            # macOS
            if command -v brew >/dev/null; then
                brew install parallel
            else
                echo "Homebrew is not installed. Please install Homebrew and try again."
                exit 1
            fi
            ;;
        *)
            echo "Unsupported operating system for automatic parallel installation."
            exit 1
            ;;
    esac
}

# Check if parallel is installed, if not, attempt to install it
if ! command -v parallel >/dev/null; then
    install_parallel
else
    echo "GNU parallel is already installed."
fi

echo 'export PYTHONPATH="$PWD:$PWD/cairo/packages/garaga_zero:$PYTHONPATH"' >> venv/bin/activate
source venv/bin/activate

pip install uv
uv pip compile cairo/packages/garaga_zero/pyproject.toml --output-file cairo/packages/garaga_zero/tools/make/requirements.txt -q
uv pip install -r cairo/packages/garaga_zero/tools/make/requirements.txt

pip install py_ecc

echo "Applying patch to instances.py..."
patch venv/lib/python3.10/site-packages/starkware/cairo/lang/instances.py < cairo/packages/garaga_zero/tools/make/instances.patch

echo "Applying patch to extension_field_modulo_circuit.py..."
patch venv/lib/python3.10/site-packages/garaga/extension_field_modulo_circuit.py < cairo/packages/garaga_zero/tools/make/extension_field_modulo_circuit.patch

deactivate

echo "Setup Complete!"