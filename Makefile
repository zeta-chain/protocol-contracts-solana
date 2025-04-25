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
.PHONY: dev
dev:
	@echo "Building gateway for development environments"
	@anchor build --program-name gateway -- --features dev

# build program for mainnet (without features)
.PHONY: prod
prod:
	@echo "Building gateway for production environments"
	@anchor build --program-name gateway

# generate Go code for development networks (with dev features)
.PHONY: generate-dev
generate-dev: dev generate

# generate Go code for mainnet
.PHONY: generate-prod
generate-prod: prod generate