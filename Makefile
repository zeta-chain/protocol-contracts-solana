# assets
C_GREEN=\033[0;32m
C_RED=\033[0;31m
C_BLUE=\033[0;34m
C_END=\033[0m

.PHONY: fmt
fmt:
	@echo "$(C_GREEN)# Formatting rust code$(C_END)"
	@./scripts/fmt.sh


# directories for Go code generation
IDL_DIR := target/idl
GENERATED_DIR := generated

# generate Go code from IDL files
.PHONY: generate
generate:
	@for file in $(wildcard $(IDL_DIR)/*.json); do \
	    base_name=$$(basename $$file .json); \
	    output_file="$(GENERATED_DIR)/$$base_name.go"; \
	    echo "Generating $$output_file from $$file"; \
	    (cd go-idl && go run generator/main.go "$$file" "$$output_file"); \
	done

# clean generated files
clean:
	rm -rf $(GENERATED_DIR)/*.go