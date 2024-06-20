import * as anchor from "@coral-xyz/anchor";
import {Program} from "@coral-xyz/anchor";
import {Gateway} from "../target/types/gateway";
import * as spl from "@solana/spl-token";
import {TOKEN_PROGRAM_ID} from "@solana/spl-token";
import { expect } from 'chai';


const fromHexString = (hexString) =>
    hexString.match(/.{1,2}/g).map((byte) => parseInt(byte, 16));


describe("some tests", () => {
    // Configure the client to use the local cluster.
    anchor.setProvider(anchor.AnchorProvider.env());
    const conn = anchor.getProvider().connection;

    const gatewayProgram = anchor.workspace.Gateway as Program<Gateway>;

    it("Test suite 1", async () => {
        let transaction = new anchor.web3.Transaction();
        const wallet = anchor.workspace.Gateway.provider.wallet.payer;
        console.log("wallet address", wallet.publicKey.toString());
        // Add your test here.
        transaction.add(await gatewayProgram.methods.initialize().instruction());
        const tx = await anchor.web3.sendAndConfirmTransaction(anchor.getProvider().connection, transaction, [wallet]);
        // console.log("Your transaction signature", tx);

        // now deploying a fake USDC SPL Token
        // 1. create a mint account
        const mint = anchor.web3.Keypair.generate();
        const mintRent = await spl.getMinimumBalanceForRentExemptMint(conn);
        const tokenTransaction = new anchor.web3.Transaction();
        tokenTransaction.add(
            anchor.web3.SystemProgram.createAccount({
                fromPubkey: wallet.publicKey,
                newAccountPubkey: mint.publicKey,
                lamports: mintRent,
                space: spl.MINT_SIZE,
                programId: spl.TOKEN_PROGRAM_ID
            }),
            spl.createInitializeMintInstruction(
                mint.publicKey,
                6,
                wallet.publicKey,
                null,
            )
        );
        await anchor.web3.sendAndConfirmTransaction(conn, tokenTransaction, [wallet, mint]);
        console.log("mint account created!", mint.publicKey.toString());

        // 2. create token account to receive mint
        const tokenAccount = await spl.getOrCreateAssociatedTokenAccount(
            conn,
            wallet,
            mint.publicKey,
            wallet.publicKey,
        );
        // 3. mint some tokens
        const mintToTransaction = new anchor.web3.Transaction().add(
            spl.createMintToInstruction(
                mint.publicKey,
                tokenAccount.address,
                wallet.publicKey,
                10_000_000,
            )
        );
        await anchor.web3.sendAndConfirmTransaction(anchor.getProvider().connection, mintToTransaction, [wallet]);
        console.log("Minted 10 USDC to:", tokenAccount.address.toString());
        const account = await spl.getAccount(conn, tokenAccount.address);
        console.log("Account balance:", account.amount.toString());
        console.log("Account owner: ", account.owner.toString());

        // OK; transfer some USDC SPL token to the gateway PDA
        const wallet_ata = await spl.getAssociatedTokenAddress(
            mint.publicKey,
            wallet.publicKey,
        );
        console.log(`wallet_ata: ${wallet_ata.toString()}`);
        let seeds = [Buffer.from("meta", "utf-8")];
        let [pdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
            seeds,
            gatewayProgram.programId,
        );
        console.log("gateway pda account", pdaAccount.toString());
        const pda_ata = await spl.getOrCreateAssociatedTokenAccount(
            conn,
            wallet,
            mint.publicKey,
            pdaAccount,
            true
        );
        console.log("pda_ata address", pda_ata.address.toString());
        const tx_xfer = await spl.transfer(
            conn,
            wallet,
            tokenAccount.address,
            pda_ata.address,
            wallet,
            1_000_000
        );
        // console.log("xfer tx hash", tx_xfer);
        const account2 = await spl.getAccount(conn, pda_ata.address);
        expect(account2.amount).to.be.eq(1_000_000n);
        // console.log("B4 withdraw: Account balance:", account2.amount.toString());


        const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
        console.log(`pda account data: nonce ${pdaAccountData.nonce}`);
        const message_hash = fromHexString(
            "0a1e2723bd7f1996832b7ed7406df8ad975deba1aa04020b5bfc3e6fe70ecc29"
        );
        const signature = fromHexString(
            "58be181f57b2d56b0c252127c9874a8fbe5ebd04f7632fb3966935a3e9a765807813692cebcbf3416cb1053ad9c8c83af471ea828242cca22076dd04ddbcd253"
        );
        const nonce = pdaAccountData.nonce;
        await gatewayProgram.methods.withdrawSplToken(new anchor.BN(500_000), signature, 0, message_hash, nonce)
            .accounts({
                from: pda_ata.address,
                to: wallet_ata,
            }).rpc();

        const account3 = await spl.getAccount(conn, pda_ata.address);
        expect(account3.amount).to.be.eq(500_000n);


        try {
            (await gatewayProgram.methods.withdrawSplToken(new anchor.BN(500_000), signature, 0, message_hash, nonce)
                .accounts({
                    from: pda_ata.address,
                    to: wallet_ata,
                }).rpc());
            throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
        } catch (err) {
            expect(err).to.be.instanceof(anchor.AnchorError);
            expect(err.message).to.include("NonceMismatch");
            const account4 = await spl.getAccount(conn, pda_ata.address);
            console.log("After 2nd withdraw: Account balance:", account4.amount.toString());
            expect(account4.amount).to.be.eq(500_000n);
        }

    });


});


