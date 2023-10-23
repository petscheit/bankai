echo "Installing and building JS packages..."
npm i && npm run build

echo "Fetching Garaga as submodule..."
git submodule update --init
echo "Moving BLS files to deps directory..."
mkdir -p cairo_programs/deps/garaga
mkdir -p cairo_programs/build
cp -R ../garaga/src cairo_programs/deps/garaga/src

echo "Creating Python VENV and installing requirements..."
python3.9 -m venv venv
echo 'export PYTHONPATH="$PWD:$PYTHONPATH"' >> venv/bin/activate
source venv/bin/activate
pip install -r scripts/requirements.txt

echo "Setup Complete!"