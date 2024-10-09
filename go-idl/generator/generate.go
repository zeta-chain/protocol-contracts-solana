package main

import (
	"fmt"
	"strings"

	"github.com/protocol-contracts-solana/go-idl/types"
)

// generateIDLFile generates the Go code for the IDL struct.
func generateIDLFile(packagename, varname string, idl *types.IDL) string {
	var sb strings.Builder

	// header
	sb.WriteString(fmt.Sprintf("package %s\n\n", packagename))
	sb.WriteString("import (\n")
	sb.WriteString("    \"github.com/protocol-contracts-solana/go-idl/types\"\n")
	sb.WriteString(")\n\n")

	// IDL struct
	sb.WriteString(fmt.Sprintf("var %s IDL = ", varname))
	sb.WriteString(generateIDLTypeCode(idl))

	return sb.String()
}

// generateIDLTypeCode generates Go code to initialize the IDL struct.
func generateIDLTypeCode(idl *types.IDL) string {
	var sb strings.Builder

	sb.WriteString(fmt.Sprintf("IDL{\n"))
	sb.WriteString(fmt.Sprintf("    Address: %q,\n", idl.Address))
	sb.WriteString(fmt.Sprintf("    Metadata: Metadata{\n"))
	sb.WriteString(fmt.Sprintf("        Name: %q,\n", idl.Metadata.Name))
	sb.WriteString(fmt.Sprintf("        Version: %q,\n", idl.Metadata.Version))
	sb.WriteString(fmt.Sprintf("        Spec: %q,\n", idl.Metadata.Spec))
	sb.WriteString(fmt.Sprintf("        Description: %q,\n", idl.Metadata.Description))
	sb.WriteString("    },\n")

	// write instructions
	sb.WriteString("    Instructions: []Instruction{\n")
	for _, instr := range idl.Instructions {
		sb.WriteString("        {\n")
		sb.WriteString(fmt.Sprintf("            Name: %q,\n", instr.Name))
		sb.WriteString(fmt.Sprintf("            Discriminator: %v,\n", instr.Discriminator))
		sb.WriteString("            Accounts: []Account{\n")
		for _, acc := range instr.Accounts {
			sb.WriteString(generateAccountCode(acc))
		}
		sb.WriteString("            },\n")
		sb.WriteString("            Args: []Arg{\n")
		for _, arg := range instr.Args {
			sb.WriteString(fmt.Sprintf("                {Name: %q, Type: %v},\n", arg.Name, arg.Type))
		}
		sb.WriteString("            },\n")
		sb.WriteString("        },\n")
	}
	sb.WriteString("    },\n")

	// write accounts
	sb.WriteString("    Accounts: []Account{\n")
	for _, acc := range idl.Accounts {
		sb.WriteString(generateAccountCode(acc))
	}
	sb.WriteString("    },\n")

	// write errors
	sb.WriteString("    Errors: []Error{\n")
	for _, err := range idl.Errors {
		sb.WriteString(fmt.Sprintf("        {Code: %d, Name: %q, Msg: %q},\n", err.Code, err.Name, err.Msg))
	}
	sb.WriteString("    },\n")

	// write types
	sb.WriteString("    Types: []Type{\n")
	for _, typ := range idl.Types {
		sb.WriteString(fmt.Sprintf("        {Name: %q, Type: TypeField{Kind: %q, Fields: []Field{\n", typ.Name, typ.Type.Kind))
		for _, field := range typ.Type.Fields {
			sb.WriteString(fmt.Sprintf("            {Name: %q, Type: %v},\n", field.Name, field.Type))
		}
		sb.WriteString("        }}},\n")
	}
	sb.WriteString("    },\n")

	sb.WriteString("}\n")
	return sb.String()
}

// generateAccountCode generates the Go code for an Account struct.
func generateAccountCode(acc types.Account) string {
	var sb strings.Builder

	sb.WriteString("                {\n")
	sb.WriteString(fmt.Sprintf("                    Name: %q,\n", acc.Name))
	sb.WriteString(fmt.Sprintf("                    Writable: %t,\n", acc.Writable))
	sb.WriteString(fmt.Sprintf("                    Signer: %t,\n", acc.Signer))
	sb.WriteString(fmt.Sprintf("                    Address: %q,\n", acc.Address))

	if acc.PDA != nil {
		sb.WriteString("                    PDA: &PDA{\n")
		sb.WriteString("                        Seeds: []Seed{\n")
		for _, seed := range acc.PDA.Seeds {
			sb.WriteString(fmt.Sprintf("                            {Kind: %q, Value: %v},\n", seed.Kind, seed.Value))
		}
		sb.WriteString("                        },\n")
		sb.WriteString("                    },\n")
	} else {
		sb.WriteString("                    PDA: nil,\n")
	}

	sb.WriteString("                },\n")
	return sb.String()
}
