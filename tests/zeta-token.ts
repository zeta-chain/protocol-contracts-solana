import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ZetaToken } from "../target/types/zeta_token";
import {
    PublicKey,
    Keypair,
    SystemProgram,
    SYSVAR_RENT_PUBKEY,
    LAMPORTS_PER_SOL
} from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    getAssociatedTokenAddress,
    createAssociatedTokenAccountInstruction,
    getAccount
} from "@solana/spl-token";
import { expect } from "chai";

describe("ZETA Token Program", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const zetaTokenProgram = anchor.workspace.ZetaToken as Program<ZetaToken>;

    // Test accounts
    let admin: Keypair;
    let user1: Keypair;
    let user2: Keypair;
    let connectorAuthority: Keypair;
    let tssAddressUpdater: Keypair;

    // PDAs
    let zetaTokenPda: PublicKey;
    let zetaMint: PublicKey;
    let zetaTokenPdaBump: number;
    let zetaMintBump: number;

    // Token accounts
    let user1TokenAccount: PublicKey;
    let user2TokenAccount: PublicKey;

    // Test parameters
    const tssAddress = new Uint8Array(20).fill(1);
    const newTssAddress = new Uint8Array(20).fill(2);
    const maxSupply = new anchor.BN(1000000000);
    const mintAmount = new anchor.BN(1000000);
    const burnAmount = new anchor.BN(100000);

    before(async () => {
        admin = Keypair.generate();
        user1 = Keypair.generate();
        user2 = Keypair.generate();
        connectorAuthority = Keypair.generate();
        tssAddressUpdater = Keypair.generate();

        const signature = await provider.connection.requestAirdrop(
            admin.publicKey,
            10 * LAMPORTS_PER_SOL
        );
        await provider.connection.confirmTransaction(signature);

        [zetaTokenPda, zetaTokenPdaBump] = PublicKey.findProgramAddressSync(
            [Buffer.from("zeta-token-pda")],
            zetaTokenProgram.programId
        );

        [zetaMint, zetaMintBump] = PublicKey.findProgramAddressSync(
            [Buffer.from("zeta-mint")],
            zetaTokenProgram.programId
        );

        user1TokenAccount = await getAssociatedTokenAddress(
            zetaMint,
            user1.publicKey
        );

        user2TokenAccount = await getAssociatedTokenAddress(
            zetaMint,
            user2.publicKey
        );
    });

    describe("Initialization", () => {
        it("Should initialize the ZETA token program", async () => {
            try {
                const tx = await zetaTokenProgram.methods
                    .initialize(
                        Array.from(tssAddress),
                        tssAddressUpdater.publicKey,
                        maxSupply
                    )
                    .accounts({
                        signer: admin.publicKey,
                        zetaTokenPda: zetaTokenPda,
                        zetaMint: zetaMint,
                        connectorAuthority: connectorAuthority.publicKey,
                        systemProgram: SystemProgram.programId,
                        tokenProgram: TOKEN_PROGRAM_ID,
                        rent: SYSVAR_RENT_PUBKEY,
                    })
                    .signers([admin, connectorAuthority])
                    .rpc();

                const pdaAccount = await zetaTokenProgram.account.zetaTokenPda.fetch(zetaTokenPda);

                expect(pdaAccount.connectorAuthority.toString()).to.equal(connectorAuthority.publicKey.toString());
                expect(Array.from(pdaAccount.tssAddress)).to.deep.equal(Array.from(tssAddress));
                expect(pdaAccount.tssAddressUpdater.toString()).to.equal(tssAddressUpdater.publicKey.toString());
                expect(pdaAccount.maxSupply.toString()).to.equal(maxSupply.toString());
                expect(pdaAccount.totalSupply.toString()).to.equal("0");
                expect(pdaAccount.decimals).to.equal(18);
            } catch (error) {
                console.error("Initialization failed: ", error);
                throw error;
            }
        });

        it("Should fail to intialize twice", async () => {
            try {
                await zetaTokenProgram.methods
                    .initialize(
                        Array.from(tssAddress),
                        tssAddressUpdater.publicKey,
                        maxSupply
                    )
                    .accounts({
                        signer: admin.publicKey,
                        zetaTokenPda: zetaTokenPda,
                        zetaMint: zetaMint,
                        connectorAuthority: connectorAuthority.publicKey,
                        systemProgram: SystemProgram.programId,
                        tokenProgram: TOKEN_PROGRAM_ID,
                        rent: SYSVAR_RENT_PUBKEY
                    })
                    .signers([admin, connectorAuthority])
                    .rpc();

                expect.fail("Should have thrown an error.")
            } catch (error) {
                expect(error).to.be.instanceOf(Error);
            }
        });
    });

    describe("TSS Address Management", () => {
        it("should update TSS address by authorized updater", async () => {
            try {
                const tx = await zetaTokenProgram.methods
                    .updateTssAddress(Array.from(newTssAddress))
                    .accounts({
                        zetaTokenPda: zetaTokenPda,
                        updater: tssAddressUpdater.publicKey
                    })
                    .signers([tssAddressUpdater])
                    .rpc();

                const pdaAccount = await zetaTokenProgram.account.zetaTokenPda.fetch(zetaTokenPda);
                expect(Array.from(pdaAccount.tssAddress)).to.deep.equal(Array.from(newTssAddress));
            } catch (error) {
                throw error;
            }
        });

        it("Should fail to update TSS address by unauthorized user", async () => {
            try {
                await zetaTokenProgram.methods
                    .updateTssAddress(Array.from(tssAddress))
                    .accounts({
                        zetaTokenPda: zetaTokenPda,
                        updater: user1.publicKey
                    })
                    .signers([user1])
                    .rpc()

                expect.fail("Should have thrown an error")
            } catch (error) {
                expect(error).to.be.instanceOf(Error);
            }
        });
    });

    describe("Token Minting", () => {
        before(async () => {
            const user1AtaIx = createAssociatedTokenAccountInstruction(
                admin.publicKey,
                user1TokenAccount,
                user1.publicKey,
                zetaMint
            );

            await provider.sendAndConfirm(
                new anchor.web3.Transaction().add(user1AtaIx),
                [admin]
            );
        });

        it("Should mint tokens to user1", async () => {
            try {
                const tx = await zetaTokenProgram.methods
                    .mint(
                        mintAmount,
                        Array.from(new Uint8Array(32).fill(1))
                    )
                    .accounts({
                        zetaTokenPda: zetaTokenPda,
                        zetaMint: zetaMint,
                        recipientTokenAccount: user1TokenAccount,
                        tokenProgram: TOKEN_PROGRAM_ID,
                        mintAuthority: connectorAuthority.publicKey
                    })
                    .signers([connectorAuthority])
                    .rpc();

                const tokenAccount = await getAccount(provider.connection, user1TokenAccount);
                expect(tokenAccount.amount.toString()).to.equal(mintAmount.toString());

                const pdaAccount = await zetaTokenProgram.account.zetaTokenPda.fetch(zetaTokenPda);
                expect(pdaAccount.totalSupply.toString()).to.equal(mintAmount.toString());
            } catch (error) {
                throw error;
            }
        });

        it("Should fail to mint with zero amount", async () => {
            try {
                await zetaTokenProgram.methods
                    .mint(
                        new anchor.BN(0),
                        Array.from(new Uint8Array(32).fill(2))
                    )
                    .accounts({
                        zetaTokenPda: zetaTokenPda,
                        zetaMint: zetaMint,
                        recipientTokenAccount: user1TokenAccount,
                        tokenProgram: TOKEN_PROGRAM_ID,
                        mintAuthority: connectorAuthority.publicKey
                    })
                    .signers([connectorAuthority])
                    .rpc();

                expect.fail("Should have thrown anerror");
            } catch (error) {
                expect(error).to.be.instanceOf(Error);
            }
        });

        it("Should fail to mint when exceeding max supply", async () => {
            try {
                const excessiveAmount = maxSupply.add(new anchor.BN(1));
                await zetaTokenProgram.methods
                    .mint(
                        excessiveAmount,
                        Array.from(new Uint8Array(32).fill(3))
                    )
                    .accounts({
                        zetaTokenPda: zetaTokenPda,
                        zetaMint: zetaMint,
                        recipientTokenAccount: user1TokenAccount,
                        tokenProgram: TOKEN_PROGRAM_ID,
                        mintAuthority: connectorAuthority.publicKey,
                    })
                    .signers([connectorAuthority])
                    .rpc();

                expect.fail("Should have thrown an error");
            } catch (error) {
                expect(error).to.be.instanceOf(Error);
            }
        });

        it("Should fail to mint with unauthorized mint authority", async () => {
            try {
                await zetaTokenProgram.methods
                    .mint(
                        mintAmount,
                        Array.from(new Uint8Array(32).fill(4))
                    )
                    .accounts({
                        zetaTokenPda: zetaTokenPda,
                        zetaMint: zetaMint,
                        recipientTokenAccount: user1TokenAccount,
                        tokenProgram: TOKEN_PROGRAM_ID,
                        mintAuthority: user1.publicKey,
                    })
                    .signers([user1])
                    .rpc();

                expect.fail("Should have thrown an error");
            } catch (error) {
                expect(error).to.be.instanceOf(Error);
            }
        });
    });

    describe("Token Burning", () => {
        before(async () => {
            const createAtaIx = createAssociatedTokenAccountInstruction(
                admin.publicKey,
                user2TokenAccount,
                user2.publicKey,
                zetaMint
            );

            await provider.sendAndConfirm(
                new anchor.web3.Transaction().add(createAtaIx),
                [admin]
            );

            await zetaTokenProgram.methods
                .mint(
                    mintAmount,
                    Array.from(new Uint8Array(32).fill(5))
                )
                .accounts({
                    zetaTokenPda: zetaTokenPda,
                    zetaMint: zetaMint,
                    recipientTokenAccount: user2TokenAccount,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    mintAuthority: connectorAuthority.publicKey,
                })
                .signers([connectorAuthority])
                .rpc();
        });

        it("Should burn tokens from user2", async () => {
            try {
                const beforeBalance = await getAccount(provider.connection, user2TokenAccount);
                const beforeTotalSupply = (await zetaTokenProgram.account.zetaTokenPda.fetch(zetaTokenPda)).totalSupply;

                const tx = await zetaTokenProgram.methods
                    .burn(burnAmount)
                    .accounts({
                        zetaTokenPda: zetaTokenPda,
                        zetaMint: zetaMint,
                        tokenAccount: user2TokenAccount,
                        tokenProgram: TOKEN_PROGRAM_ID,
                        burnAuthority: connectorAuthority.publicKey,
                    })
                    .signers([connectorAuthority])
                    .rpc();

                console.log("Burn transaction signature:", tx);

                const afterBalance = await getAccount(provider.connection, user2TokenAccount);
                expect(afterBalance.amount.toString()).to.equal(
                    beforeBalance.amount.sub(burnAmount).toString()
                );

                const afterTotalSupply = (await zetaTokenProgram.account.zetaTokenPda.fetch(zetaTokenPda)).totalSupply;
                expect(afterTotalSupply.toString()).to.equal(
                    beforeTotalSupply.sub(burnAmount).toString()
                );
            } catch (error) {
                throw error;
            }
        });

        it("Should fail to burn zero tokens", async () => {
            try {
                await zetaTokenProgram.methods
                    .burn(new anchor.BN(0))
                    .accounts({
                        zetaTokenPda: zetaTokenPda,
                        zetaMint: zetaMint,
                        tokenAccount: user2TokenAccount,
                        tokenProgram: TOKEN_PROGRAM_ID,
                        burnAuthority: connectorAuthority.publicKey,
                    })
                    .signers([connectorAuthority])
                    .rpc();

                expect.fail("Should have thrown an error");
            } catch (error) {
                expect(error).to.be.instanceOf(Error);
            }
        });

        it("Should fail to burn with unauthorized burn authority", async () => {
            try {
                await zetaTokenProgram.methods
                    .burn(burnAmount)
                    .accounts({
                        zetaTokenPda: zetaTokenPda,
                        zetaMint: zetaMint,
                        tokenAccount: user2TokenAccount,
                        tokenProgram: TOKEN_PROGRAM_ID,
                        burnAuthority: user2.publicKey,
                    })
                    .signers([user2])
                    .rpc();

                expect.fail("Should have thrown an error");
            } catch (error) {
                expect(error).to.be.instanceOf(Error);
            }
        });
    });
});