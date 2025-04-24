# directories for Go code generation
IDL_DIR := ./target/idl
GENERATED_DIR := ./generated
# Default version if not specified
VERSION ?= 0.1.0

# generate Go bindings from IDL files
.PHONY: generate
generate:
	@chmod +x ./scripts/generate_go_bindings.sh
	@./scripts/generate_go_bindings.sh $(IDL_DIR) $(GENERATED_DIR)

# build program with dev features for testnet
.PHONY: testnet
testnet:
	@echo "Building gateway with dev features..."
	@anchor build --program-name gateway -- --features dev

# build program for mainnet (without features)
.PHONY: mainnet
mainnet:
	@echo "Building gateway for mainnet..."
	@anchor build --program-name gateway

# generate Go code for testnet (with dev features)
.PHONY: generate-testnet
generate-testnet: testnet generate

# generate Go code for mainnet
.PHONY: generate-mainnet
generate-mainnet: mainnet generate