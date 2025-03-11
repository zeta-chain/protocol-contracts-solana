package types

// IDL represents the interface definition language for a Solana program
// more info: https://solana.com/hi/docs/programs/anchor/idl
type IDL struct {
	Address      string        `json:"address"`
	Metadata     Metadata      `json:"metadata"`
	Instructions []Instruction `json:"instructions"`
	Accounts     []Account     `json:"accounts"`
	Errors       []Error       `json:"errors"`
	Types        []Type        `json:"types"`
}

// GetDiscriminator returns the discriminator from the instruction name
func (i IDL) GetDiscriminator(name string) [8]byte {
	for _, instr := range i.Instructions {
		if instr.Name == name {
			return instr.Discriminator
		}
	}
	return [8]byte{}
}

// Metadata represents the metadata of the IDL
type Metadata struct {
	Name        string `json:"name"`
	Version     string `json:"version"`
	Spec        string `json:"spec"`
	Description string `json:"description"`
}

// Instruction represents a single instruction in the IDL
type Instruction struct {
	Name          string    `json:"name"`
	Discriminator [8]byte   `json:"discriminator"`
	Accounts      []Account `json:"accounts"`
	Args          []Arg     `json:"args"`
}

// Account represents an account in the IDL
type Account struct {
	Name     string `json:"name"`
	Writable bool   `json:"writable,omitempty"`
	Signer   bool   `json:"signer,omitempty"`
	Address  string `json:"address,omitempty"`
	PDA      *PDA   `json:"pda,omitempty"`
}

// PDA represents a program-derived address in the IDL
type PDA struct {
	Seeds []Seed `json:"seeds"`
}

// Seed represents a seed in the PDA
type Seed struct {
	Kind  string `json:"kind"`
	Value []byte `json:"value,omitempty"`
}

// Arg represents an argument in the IDL
type Arg struct {
	Name string      `json:"name"`
	Type interface{} `json:"type"`
}

// Error represents an error in the IDL
type Error struct {
	Code int    `json:"code"`
	Name string `json:"name"`
	Msg  string `json:"msg"`
}

// Type represents a type in the IDL
type Type struct {
	Name string    `json:"name"`
	Type TypeField `json:"type"`
}

// TypeField represents a field in a type in the IDL
type TypeField struct {
	Kind   string  `json:"kind"`
	Fields []Field `json:"fields"`
}

// Field represents a field in a type in the IDL
type Field struct {
	Name string      `json:"name"`
	Type interface{} `json:"type"`
}
