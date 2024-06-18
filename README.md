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

# Build and Test Instructions

Prerequisites: a recent version of `rust` compiler
and `cargo` package manger must be installed. The program
is built with the `anchor` framework so it needs to be
installed as well; see [installation](https://www.anchor-lang.com/docs/installation)

To build
```bash
$ anchor build
```

To run the tests
```bash
$ anchor test
```