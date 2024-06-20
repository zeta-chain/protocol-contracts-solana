
# Introduction

This repository hosts the smart contracts (program)
on Solana network to support ZetaChain cross-chain
functionality. Specifically, it consists of a single
program deployed (to be deployed; show address here)
which allows the following two actions: 

1. Users on Solana network can send SOL or selected
SPL tokens to the program to deposit into ZetaChain
and optionally invoke a ZetaChain EVM contract. 
2. Allows contracts on ZetaChain EVM to withdraw
SOL and SPL tokens to users on Solana;
3. (TO DO) In the withdraw above, optionally allow
a contract on ZetaChain EVM to withdraw SOL/SPL tokens
and call a user specified contract (program) with
parameters. 

# Authentication and Authorization

Anyone can deposit and remote invoke ZetaChain contracts. 

Only ZetaChain TSS account can make withdraw or withdrawAndCall
transactions on the program. The ZetaChain TSS account
is a collection of Observer/KeySigners which uses
ECDSA TSS (Threshold Signature Scheme) to sign 
outbound transactions. The TSS address will appear in this program
a single ECDSA secp256k1 address; but it does
not have a single private key, rather its private
key consists of multiple key shares and they collectively
sign a message in a KeySign MPC ceremony. 
The program authenticates
via verifying the TSS signature and is authorized
by ZetaChain to perform outbound transactions as
part of ZetaChain cross-chain machinery. 

The ZetaChain TSS is on ECDSA secp256k1 curve; 
But Solana native digital signature scheme is
EdDSA Ed25519 curve.  Therefore the program uses
custom logic to verify the TSS ECDSA signature
(like alternative authentication in smart contract wallet); 
the native transaction signer (fee payer on Solana)
does not carry authorization and it's only used
to build the transaction and pay tx fees. There
are no restrictions on who the native transaction
signer/fee payer is. 

In both `withdraw` and `withdrawAndCall` instructions, 
the ECDSA signed message_hash must commit to the 
`nonce`, `amount`, and recipient address. See the 
check in these instructions like: 
```rust
let mut concatenated_buffer = Vec::new();
concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
concatenated_buffer.extend_from_slice(&amount.to_be_bytes());
concatenated_buffer.extend_from_slice(&ctx.accounts.to.key().to_bytes());
require!(message_hash == hash(&concatenated_buffer[..]).to_bytes(), Errors::MessageHashMismatch);
```

# Build and Test Instructions

Prerequisites: a recent version of `rust` compiler
and `cargo` package manger must be installed. The program
is built with the `anchor` framework so it needs to be
installed as well; see [installation](https://www.anchor-lang.com/docs/installation)

To build (this will require at least 2GB disk space)
```bash
$ anchor build
```

To run the tests
```bash
$ anchor test
```

# Relevant Account and Addresses

The Gateway program derive a PDA (Program Derived Address)
with seeds `b"meta"` and canonical bump. 
This PDA account/address actually holds the SOL
balance of the Gateway program. 
For SPL tokens, the program stores the SPL token
in PDA derived ATAs. For each SPL token (different mint
account), the program creates ATA from PDA and the Mint
(standard way of deriving ATA in Solana SPL program).

The PDA account itself is a data account that holds
Gateway program state, namely the following data types
```rust
pub struct Pda {
    nonce: u64,            // ensure that each signature can only be used once
    tss_address: [u8; 20], // 20 bytes address format of ethereum
}
```
The `nonce` is incremented on each successful withdraw transaction,
and it's used to prevent replay of signed ECDSA messages. 
The `tss_address` is the TSS address of ZetaChain (20Bytes,
Ethereum style). 

The `initialize` instruction sets nonce to 0. 