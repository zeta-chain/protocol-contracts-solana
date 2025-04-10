#!/bin/bash
# generate_idl.sh

# Get parameters
IDL_DIR=$1
GENERATED_DIR=$2

# Remove existing generated files
rm -rf ${GENERATED_DIR}/*.go

# Process each IDL file
for file in $(find ${IDL_DIR} -name "*.json"); do
    base_name=$(basename $file .json)
    input_file="../$file"
    output_file="${GENERATED_DIR}/${base_name}.go"

    echo "Generating ${output_file} from ${file}"

    # Run the generator
    (cd go-idl && go run ./generator "${input_file}" "${output_file}")

    # Format the generated file
    (cd go-idl && go fmt "${output_file}")
done