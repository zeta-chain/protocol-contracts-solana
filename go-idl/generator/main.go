package main

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/zeta-chain/protocol-contracts-solana/go-idl/types"
)

func main() {
	// ensure the correct number of arguments.
	if len(os.Args) < 3 {
		fmt.Println("Usage: go run main.go <path-to-idl.json> <output-path>")
		os.Exit(1)
	}

	inputPath := os.Args[1]
	outputPath := os.Args[2]

	// parse the IDL from the input JSON file.
	idl, err := readIDL(inputPath)
	if err != nil {
		fmt.Printf("Error reading IDL: %v\n", err)
		os.Exit(1)
	}

	// write the parsed IDL as a Go constant in `generated.go`.
	err = writeGoFile(idl, outputPath)
	if err != nil {
		fmt.Printf("Error writing Go file: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Successfully generated Go file: %s\n", outputPath)
}

// readIDL reads the idl.json file and parses it into an IDL struct.
func readIDL(filePath string) (*types.IDL, error) {
	// read the JSON file
	data, err := os.ReadFile(filePath)
	if err != nil {
		return nil, fmt.Errorf("failed to read file: %w", err)
	}

	// parse the JSON data into an IDL struct
	var idl types.IDL
	if err := json.Unmarshal(data, &idl); err != nil {
		return nil, fmt.Errorf("failed to parse JSON: %w", err)
	}

	return &idl, nil
}

// writeGoFile generates the `generated.go` file with the constant `IDL`.
func writeGoFile(idl *types.IDL, outputPath string) error {
	// create the output file
	file, err := os.Create(outputPath)
	if err != nil {
		return fmt.Errorf("failed to create file: %w", err)
	}

	// write the Go code to the file
	_, err = file.WriteString(generateIDLFile("solana", "Gateway", idl))
	if err != nil {
		return fmt.Errorf("failed to write to file: %w", err)
	}

	// close the file
	if err := file.Close(); err != nil {
		return fmt.Errorf("failed to close file: %w", err)
	}

	return nil
}
