# assets
C_GREEN=\033[0;32m
C_RED=\033[0;31m
C_BLUE=\033[0;34m
C_END=\033[0m

.PHONY: fmt
fmt:
	@echo "$(C_GREEN)# Formatting rust code$(C_END)"
	@./scripts/fmt.sh


# Variables
GENERATOR_DIR := go-idl/generator
IDL_FILE := target/idl/gateway.json
OUTPUT_DIR := go-idl/generated/gateway.go

# Generate go code
generate:
	go run $(GENERATOR_DIR) $(IDL_FILE) $(OUTPUT_DIR)