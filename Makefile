# directories for Go code generation
IDL_DIR := ./target/idl
GENERATED_DIR := ./generated

# generate Go code from IDL files
.PHONY: generate
generate:
	rm -rf $(GENERATED_DIR)/*.go
	@for file in $(wildcard $(IDL_DIR)/*.json); do \
	    base_name=$$(basename $$file .json); \
	    input_file="../$$file"; \
	    output_file="$(GENERATED_DIR)/$$base_name/$$base_name.go"; \
	    echo "Generating $$output_file from $$file"; \
	    (cd go-idl && go run ./generator "$$input_file" "$$output_file"); \
	    (cd go-idl && go fmt $$output_file); \
	done
