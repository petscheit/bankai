#!/bin/bash

# Exit on any error
set -e

# Variables
DATA_DIR="./nimbus-data"
EXECUTION_STUB_IMAGE="chainsafe/lodestar:latest"
NIMBUS_IMAGE="statusim/nimbus-eth2:multiarch-latest"
EXECUTION_STUB_CONTAINER="execution-stub"
NIMBUS_CONTAINER="nimbus-sepolia"
EXECUTION_ENDPOINT="http://localhost:8545"
REST_PORT=5052
BEACON_PORT=9000
JWT_SECRET_FILE="$DATA_DIR/jwt-secret"
TRUSTED_NODE_URL="https://sepolia.beaconstate.info"

# Create a data directory for Nimbus if it doesn't exist
if [ ! -d "$DATA_DIR" ]; then
  mkdir -p "$DATA_DIR"
  chmod 700 "$DATA_DIR"
  echo "Created data directory: $DATA_DIR with secure permissions"
fi

# Pull the latest Docker images
echo "Pulling latest Docker images..."
docker pull $EXECUTION_STUB_IMAGE
docker pull $NIMBUS_IMAGE

# Start the unsafe execution stub
echo "Starting the unsafe execution stub..."
docker run -d --rm \
  --name $EXECUTION_STUB_CONTAINER \
  -p 8545:8545 \
  $EXECUTION_STUB_IMAGE \
  node ./packages/execution/stub

# Wait a few seconds to ensure the execution stub is running
sleep 5
echo "Execution stub is running on $EXECUTION_ENDPOINT"

# Create a JWT secret file if it doesn't exist
if [ ! -f "$JWT_SECRET_FILE" ]; then
  echo "Creating JWT secret file..."
  echo "0x7365637265747365637265747365637265747365637265747365637265747365" > "$JWT_SECRET_FILE"
fi

# Perform trusted node sync
echo "Performing trusted node sync..."
if ! docker run --rm \
  -v "$(pwd)/$DATA_DIR:/data" \
  $NIMBUS_IMAGE \
  trustedNodeSync \
  --network=sepolia \
  --data-dir=/data \
  --trusted-node-url=$TRUSTED_NODE_URL; then
    echo "Trusted node sync failed. Please check the endpoint and try again."
    exit 1
fi

echo "Trusted node sync completed."

# Start the Nimbus beacon node
echo "Starting the Nimbus beacon node..."
docker run --rm \
  --name $NIMBUS_CONTAINER \
  -v "$(pwd)/$DATA_DIR:/data" \
  -p $REST_PORT:$REST_PORT \
  -p $BEACON_PORT:$BEACON_PORT \
  $NIMBUS_IMAGE \
  --network=sepolia \
  --data-dir=/data \
  --jwt-secret=/data/jwt-secret \
  --el=http://host.docker.internal:8545 \
  --rest \
  --rest-port=$REST_PORT \
  --rest-address=0.0.0.0

# Increase wait time to ensure Nimbus is fully initialized
sleep 15

# Confirm that both containers are running
echo "Checking running containers..."
docker ps --filter "name=$EXECUTION_STUB_CONTAINER" --filter "name=$NIMBUS_CONTAINER"

echo "Setup complete!"
echo "Nimbus REST API is available at: http://localhost:$REST_PORT"
