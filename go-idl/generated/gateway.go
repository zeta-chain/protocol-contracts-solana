// Code generated by go-idl. DO NOT EDIT.
package solana

import (
	"github.com/zeta-chain/protocol-contracts-solana/go-idl/types"
)

var IDLGateway = types.IDL{
	Address: "ZETAjseVjuFsxdRxo6MmTCvqFwb3ZHUx56Co3vCmGis",
	Metadata: types.Metadata{
		Name:        "gateway",
		Version:     "0.1.0",
		Spec:        "0.1.0",
		Description: "Created with Anchor",
	},
	Instructions: []types.Instruction{
		{
			Name:          "deposit",
			Discriminator: [8]byte{242, 35, 198, 137, 82, 225, 242, 182},
			Accounts: []types.Account{
				{
					Name:     "signer",
					Writable: true,
					Signer:   true,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "pda",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "system_program",
					Writable: false,
					Signer:   false,
					Address:  "11111111111111111111111111111111",
					PDA:      nil,
				},
			},
		},
		{
			Name:          "deposit_and_call",
			Discriminator: [8]byte{65, 33, 186, 198, 114, 223, 133, 57},
			Accounts: []types.Account{
				{
					Name:     "signer",
					Writable: true,
					Signer:   true,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "pda",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "system_program",
					Writable: false,
					Signer:   false,
					Address:  "11111111111111111111111111111111",
					PDA:      nil,
				},
			},
		},
		{
			Name:          "deposit_spl_token",
			Discriminator: [8]byte{86, 172, 212, 121, 63, 233, 96, 144},
			Accounts: []types.Account{
				{
					Name:     "signer",
					Writable: true,
					Signer:   true,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "pda",
					Writable: false,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "whitelist_entry",
					Writable: false,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "mint_account",
					Writable: false,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "token_program",
					Writable: false,
					Signer:   false,
					Address:  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
					PDA:      nil,
				},
				{
					Name:     "from",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "to",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
			},
		},
		{
			Name:          "deposit_spl_token_and_call",
			Discriminator: [8]byte{14, 181, 27, 187, 171, 61, 237, 147},
			Accounts: []types.Account{
				{
					Name:     "signer",
					Writable: true,
					Signer:   true,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "pda",
					Writable: false,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "whitelist_entry",
					Writable: false,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "mint_account",
					Writable: false,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "token_program",
					Writable: false,
					Signer:   false,
					Address:  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
					PDA:      nil,
				},
				{
					Name:     "from",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "to",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
			},
		},
		{
			Name:          "initialize",
			Discriminator: [8]byte{175, 175, 109, 31, 13, 152, 155, 237},
			Accounts: []types.Account{
				{
					Name:     "signer",
					Writable: true,
					Signer:   true,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "pda",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "system_program",
					Writable: false,
					Signer:   false,
					Address:  "11111111111111111111111111111111",
					PDA:      nil,
				},
			},
		},
		{
			Name:          "set_deposit_paused",
			Discriminator: [8]byte{98, 179, 141, 24, 246, 120, 164, 143},
			Accounts: []types.Account{
				{
					Name:     "pda",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "signer",
					Writable: true,
					Signer:   true,
					Address:  "",
					PDA:      nil,
				},
			},
		},
		{
			Name:          "unwhitelist_spl_mint",
			Discriminator: [8]byte{73, 142, 63, 191, 233, 238, 170, 104},
			Accounts: []types.Account{
				{
					Name:     "whitelist_entry",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "whitelist_candidate",
					Writable: false,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "pda",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "authority",
					Writable: true,
					Signer:   true,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "system_program",
					Writable: false,
					Signer:   false,
					Address:  "11111111111111111111111111111111",
					PDA:      nil,
				},
			},
		},
		{
			Name:          "update_authority",
			Discriminator: [8]byte{32, 46, 64, 28, 149, 75, 243, 88},
			Accounts: []types.Account{
				{
					Name:     "pda",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "signer",
					Writable: true,
					Signer:   true,
					Address:  "",
					PDA:      nil,
				},
			},
		},
		{
			Name:          "update_tss",
			Discriminator: [8]byte{227, 136, 3, 242, 177, 168, 10, 160},
			Accounts: []types.Account{
				{
					Name:     "pda",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "signer",
					Writable: true,
					Signer:   true,
					Address:  "",
					PDA:      nil,
				},
			},
		},
		{
			Name:          "whitelist_spl_mint",
			Discriminator: [8]byte{30, 110, 162, 42, 208, 147, 254, 219},
			Accounts: []types.Account{
				{
					Name:     "whitelist_entry",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "whitelist_candidate",
					Writable: false,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "pda",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "authority",
					Writable: true,
					Signer:   true,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "system_program",
					Writable: false,
					Signer:   false,
					Address:  "11111111111111111111111111111111",
					PDA:      nil,
				},
			},
		},
		{
			Name:          "withdraw",
			Discriminator: [8]byte{183, 18, 70, 156, 148, 109, 161, 34},
			Accounts: []types.Account{
				{
					Name:     "signer",
					Writable: true,
					Signer:   true,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "pda",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "to",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
			},
		},
		{
			Name:          "withdraw_spl_token",
			Discriminator: [8]byte{219, 156, 234, 11, 89, 235, 246, 32},
			Accounts: []types.Account{
				{
					Name:     "signer",
					Writable: true,
					Signer:   true,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "pda",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "pda_ata",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "mint_account",
					Writable: false,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "to",
					Writable: true,
					Signer:   false,
					Address:  "",
					PDA:      nil,
				},
				{
					Name:     "token_program",
					Writable: false,
					Signer:   false,
					Address:  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
					PDA:      nil,
				},
			},
		},
	},
	Accounts: []types.Account{
		{
			Name:     "Pda",
			Writable: false,
			Signer:   false,
			Address:  "",
			PDA:      nil,
		},
		{
			Name:     "WhitelistEntry",
			Writable: false,
			Signer:   false,
			Address:  "",
			PDA:      nil,
		},
	},
	Errors: []types.Error{
		{Code: 6000, Name: "SignerIsNotAuthority", Msg: "SignerIsNotAuthority"},
		{Code: 6001, Name: "InsufficientPoints", Msg: "InsufficientPoints"},
		{Code: 6002, Name: "NonceMismatch", Msg: "NonceMismatch"},
		{Code: 6003, Name: "TSSAuthenticationFailed", Msg: "TSSAuthenticationFailed"},
		{Code: 6004, Name: "DepositToAddressMismatch", Msg: "DepositToAddressMismatch"},
		{Code: 6005, Name: "MessageHashMismatch", Msg: "MessageHashMismatch"},
		{Code: 6006, Name: "MemoLengthExceeded", Msg: "MemoLengthExceeded"},
		{Code: 6007, Name: "MemoLengthTooShort", Msg: "MemoLengthTooShort"},
		{Code: 6008, Name: "DepositPaused", Msg: "DepositPaused"},
		{Code: 6009, Name: "SPLAtaAndMintAddressMismatch", Msg: "SPLAtaAndMintAddressMismatch"},
	},
}

const (
	InstructionDeposit                    = "deposit"
	InstructionDeposit_and_call           = "deposit_and_call"
	InstructionDeposit_spl_token          = "deposit_spl_token"
	InstructionDeposit_spl_token_and_call = "deposit_spl_token_and_call"
	InstructionInitialize                 = "initialize"
	InstructionSet_deposit_paused         = "set_deposit_paused"
	InstructionUnwhitelist_spl_mint       = "unwhitelist_spl_mint"
	InstructionUpdate_authority           = "update_authority"
	InstructionUpdate_tss                 = "update_tss"
	InstructionWhitelist_spl_mint         = "whitelist_spl_mint"
	InstructionWithdraw                   = "withdraw"
	InstructionWithdraw_spl_token         = "withdraw_spl_token"
)
