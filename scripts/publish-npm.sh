#!/bin/bash
# Script to build and publish Gateway program to NPM

# Default variables
PACKAGE_NAME="@zetachain/gateway"
TEMP_DIR="./npm-package"
MAINNET_PROGRAM_ID="ZETAjseVjuFsxdRxo6MmTCvqFwb3ZHUx56Co3vCmGis"
TESTNET_PROGRAM_ID="94U5AHQMKkV5txNJ17QPXWoh474PheGou6cNP2FEuL1d"

# Check if version was provided as an argument
if [ $# -eq 0 ]; then
  echo "Error: No version specified"
  echo "Usage: $0 <version>"
  echo "Example: $0 0.1.0"
  exit 1
fi

# Set version from argument
PACKAGE_VERSION="$1"
echo "Building and publishing version $PACKAGE_VERSION"

# Clean any existing temp directory
rm -rf $TEMP_DIR

echo "Building mainnet version..."
anchor run build-gateway

echo "Creating mainnet directories..."
mkdir -p $TEMP_DIR/mainnet/lib
mkdir -p $TEMP_DIR/mainnet/idl
cp target/deploy/gateway.so $TEMP_DIR/mainnet/lib/
cp target/idl/gateway.json $TEMP_DIR/mainnet/idl/gateway.json

echo "Building testnet version..."
anchor run build-gateway-dev

echo "Creating testnet directories..."
mkdir -p $TEMP_DIR/testnet/lib
mkdir -p $TEMP_DIR/testnet/idl
cp target/deploy/gateway.so $TEMP_DIR/testnet/lib/
cp target/idl/gateway.json $TEMP_DIR/testnet/idl/gateway.json

echo "Creating package.json..."
cat > $TEMP_DIR/package.json << EOF
{
  "name": "$PACKAGE_NAME",
  "version": "$PACKAGE_VERSION",
  "description": "Gateway program and IDL files for mainnet and testnet",
  "main": "index.js",
  "files": ["mainnet", "testnet", "index.js"],
  "keywords": ["gateway", "zetachain"],
  "author": "ZetaChain",
  "license": "MIT"
}
EOF

echo "Creating index.js..."
cat > $TEMP_DIR/index.js << EOF
module.exports = {
  mainnet: {
    programId: "$MAINNET_PROGRAM_ID",
    idl: require("./mainnet/idl/gateway.json")
  },
  testnet: {
    programId: "$TESTNET_PROGRAM_ID",
    idl: require("./testnet/idl/gateway.json")
  }
};
EOF

echo "Publishing to NPM..."
cd $TEMP_DIR && npm publish

echo "Cleaning up..."
cd ..
rm -rf $TEMP_DIR

echo "Successfully published version $PACKAGE_VERSION to NPM!"