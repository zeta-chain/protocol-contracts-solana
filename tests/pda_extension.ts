import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Gateway } from "../target/types/gateway";
import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";

describe("PDA Extension Tests", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.Gateway as Program<Gateway>;
    const wallet = provider.wallet;

    // Test constants
    const chainId = 111111;
    const tssAddress = new Array(20).fill(0).map(() => Math.floor(Math.random() * 256));
    const bump = 255; // Example bump value

    let pda: PublicKey;
    let pdaBump: number;

    before(async () => {
        // Find the PDA
        [pda, pdaBump] = PublicKey.findProgramAddressSync(
            [Buffer.from("meta")],
            program.programId
        );
    });

    it("Initializes the gateway PDA", async () => {
        try {
            const tx = await program.methods
                .initialize(tssAddress, new anchor.BN(chainId))
                .accounts({
                    signer: wallet.publicKey,
                    pda: pda,
                    systemProgram: anchor.web3.SystemProgram.programId,
                })
                .rpc();

            console.log("Initialize transaction signature:", tx);

            // Verify the PDA was initialized
            const pdaAccount = await program.account.pda.fetch(pda);
            expect(pdaAccount.nonce.toString()).to.equal("0");
            expect(pdaAccount.chainId.toString()).to.equal(chainId.toString());
            expect(pdaAccount.authority.toString()).to.equal(wallet.publicKey.toString());
            expect(pdaAccount.depositPaused).to.be.false;
            expect(Array.from(pdaAccount.tssAddress)).to.deep.equal(tssAddress);

            console.log("PDA initialized successfully");
        } catch (error) {
            console.error("Error initializing PDA:", error);
            throw error;
        }
    });

    it("Extends the PDA with new fields using realloc", async () => {
        try {
            // Get the current PDA account size
            const pdaAccountInfo = await program.provider.connection.getAccountInfo(pda);
            const currentSize = pdaAccountInfo?.data.length || 0;
            console.log("Current PDA size:", currentSize);

            // Extend the PDA
            const tx = await program.methods
                .extendPda(bump)
                .accounts({
                    authority: wallet.publicKey,
                    pda: pda,
                    systemProgram: anchor.web3.SystemProgram.programId,
                })
                .rpc();

            console.log("Extend PDA transaction signature:", tx);

            // Verify the PDA was extended
            const pdaAccountInfoAfter = await program.provider.connection.getAccountInfo(pda);
            const newSize = pdaAccountInfoAfter?.data.length || 0;

            console.log("New PDA size:", newSize);
            expect(newSize).to.be.greaterThan(currentSize);
            expect(newSize - currentSize).to.equal(2); // 2 bytes for bump and version fields

            // Verify the PDA still works by fetching it
            const pdaAccount = await program.account.pda.fetch(pda);
            expect(pdaAccount.nonce.toString()).to.equal("0");
            expect(pdaAccount.chainId.toString()).to.equal(chainId.toString());
            expect(pdaAccount.authority.toString()).to.equal(wallet.publicKey.toString());
            expect(pdaAccount.depositPaused).to.be.false;
            expect(Array.from(pdaAccount.tssAddress)).to.deep.equal(tssAddress);

            console.log("PDA extended successfully");
        } catch (error) {
            console.error("Error extending PDA:", error);
            throw error;
        }
    });

    it("Verifies PDA functionality after extension", async () => {
        try {
            // Test that the PDA still works for normal operations
            const newTssAddress = new Array(20).fill(1).map(() => Math.floor(Math.random() * 256));

            const tx = await program.methods
                .updateTss(newTssAddress)
                .accounts({
                    signer: wallet.publicKey,
                    pda: pda,
                })
                .rpc();

            console.log("Update TSS transaction signature:", tx);

            // Verify the update worked
            const pdaAccount = await program.account.pda.fetch(pda);
            expect(Array.from(pdaAccount.tssAddress)).to.deep.equal(newTssAddress);
            expect(pdaAccount.nonce.toString()).to.equal("0"); // Should be reset after TSS update

            console.log("PDA functionality verified after extension");
        } catch (error) {
            console.error("Error verifying PDA functionality:", error);
            throw error;
        }
    });

    it("Tests rent exemption after extension", async () => {
        try {
            // Check that the PDA is still rent-exempt after extension
            const pdaAccountInfo = await program.provider.connection.getAccountInfo(pda);
            const lamports = pdaAccountInfo?.lamports || 0;
            const dataLength = pdaAccountInfo?.data.length || 0;

            // Calculate minimum balance for rent exemption
            const rent = await program.provider.connection.getMinimumBalanceForRentExemption(dataLength);

            console.log("PDA lamports:", lamports);
            console.log("Minimum rent-exempt balance:", rent);
            console.log("PDA data length:", dataLength);

            expect(lamports).to.be.greaterThanOrEqual(rent);

            console.log("PDA is rent-exempt after extension");
        } catch (error) {
            console.error("Error checking rent exemption:", error);
            throw error;
        }
    });

    it("Tests multiple extensions", async () => {
        try {
            // Test extending the PDA multiple times
            const initialSize = (await program.provider.connection.getAccountInfo(pda))?.data.length || 0;

            // First extension
            await program.methods
                .extendPda(200)
                .accounts({
                    authority: wallet.publicKey,
                    pda: pda,
                    systemProgram: anchor.web3.SystemProgram.programId,
                })
                .rpc();

            const sizeAfterFirst = (await program.provider.connection.getAccountInfo(pda))?.data.length || 0;
            expect(sizeAfterFirst).to.be.greaterThan(initialSize);

            // Second extension
            await program.methods
                .extendPda(150)
                .accounts({
                    authority: wallet.publicKey,
                    pda: pda,
                    systemProgram: anchor.web3.SystemProgram.programId,
                })
                .rpc();

            const sizeAfterSecond = (await program.provider.connection.getAccountInfo(pda))?.data.length || 0;
            expect(sizeAfterSecond).to.be.greaterThan(sizeAfterFirst);

            console.log("Multiple extensions successful");
        } catch (error) {
            console.error("Error with multiple extensions:", error);
            throw error;
        }
    });

    it("Tests unauthorized access prevention", async () => {
        try {
            // Create a different wallet to test unauthorized access
            const unauthorizedWallet = anchor.web3.Keypair.generate();

            // Fund the unauthorized wallet
            const signature = await program.provider.connection.requestAirdrop(
                unauthorizedWallet.publicKey,
                2 * anchor.web3.LAMPORTS_PER_SOL
            );
            await program.provider.connection.confirmTransaction(signature);

            // Try to extend PDA with unauthorized wallet
            try {
                await program.methods
                    .extendPda(100)
                    .accounts({
                        authority: unauthorizedWallet.publicKey,
                        pda: pda,
                        systemProgram: anchor.web3.SystemProgram.programId,
                    })
                    .signers([unauthorizedWallet])
                    .rpc();

                // If we get here, the test should fail
                expect.fail("Unauthorized access should have been prevented");
            } catch (error) {
                // This is expected - unauthorized access should be prevented
                expect(error.message).to.include("SignerIsNotAuthority");
                console.log("Unauthorized access correctly prevented");
            }
        } catch (error) {
            console.error("Error testing unauthorized access:", error);
            throw error;
        }
    });
});
