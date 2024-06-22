import * as anchor from "@coral-xyz/anchor";
import {Program, web3} from "@coral-xyz/anchor";
import {Gateway} from "../target/types/gateway";
import * as spl from "@solana/spl-token";
import * as memo from "@solana/spl-memo";
import {randomFillSync} from 'crypto';
import { ec as EC } from 'elliptic';
import { keccak256 } from 'ethereumjs-util';
import { bufferToHex } from 'ethereumjs-util';
import {expect} from 'chai';
import {ecdsaRecover} from 'secp256k1';



const ec = new EC('secp256k1');
const keyPair = ec.genKeyPair();
console.log("private key", keyPair.getPrivate('hex'));

describe("some tests", () => {
    // Configure the client to use the local cluster.
    anchor.setProvider(anchor.AnchorProvider.env());
    const conn = anchor.getProvider().connection;
    const gatewayProgram = anchor.workspace.Gateway as Program<Gateway>;
    const wallet = anchor.workspace.Gateway.provider.wallet.payer;
    const mint = anchor.web3.Keypair.generate();
    let tokenAccount: spl.Account;
    let wallet_ata: anchor.web3.PublicKey;
    let pdaAccount: anchor.web3.PublicKey;
    let pda_ata: spl.Account;
    const message_hash = keccak256(Buffer.from("hello world"));
    const signature = keyPair.sign(message_hash, 'hex');
    const { r, s, recoveryParam } = signature;
    console.log("r", recoveryParam);
    const signatureBuffer = Buffer.concat([
        r.toArrayLike(Buffer, 'be', 32),
        s.toArrayLike(Buffer, 'be', 32),
    ]);
    const recoveredPubkey = ecdsaRecover(signatureBuffer, recoveryParam, message_hash, false);
    console.log("recovered pubkey    ", bufferToHex(Buffer.from(recoveredPubkey)));
    const publicKeyBuffer = Buffer.from(keyPair.getPublic(false, 'hex').slice(2), 'hex');  // Uncompressed form of public key, remove the '04' prefix
    console.log("generated public key", bufferToHex(publicKeyBuffer));

    const addressBuffer = keccak256(publicKeyBuffer);  // Skip the first byte (format indicator)
    const address = addressBuffer.slice(-20);
    console.log("address", bufferToHex(address));
    // const tssAddress = [239, 36, 74, 232, 12, 58, 220, 53, 101, 185, 127, 45, 0, 144, 15, 163, 104, 163, 74, 178,];
    const tssAddress = Array.from(address);

    it("Initializes the program", async () => {
        await gatewayProgram.methods.initialize(tssAddress).rpc();

        // repeated initialization should fail
        try {
            await gatewayProgram.methods.initialize(tssAddress).rpc();
            throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
        } catch (err) {
            expect(err).to.be.not.null;
            // console.log("Error message: ", err.message)
        }
    });

    it("Mint a SPL USDC token", async () => {
        // now deploying a fake USDC SPL Token
        // 1. create a mint account
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
        tokenAccount = await spl.getOrCreateAssociatedTokenAccount(
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
        wallet_ata = await spl.getAssociatedTokenAddress(
            mint.publicKey,
            wallet.publicKey,
        );
        console.log(`wallet_ata: ${wallet_ata.toString()}`);
    })

    it("Deposit 1_000_000 USDC to Gateway", async () => {
        let seeds = [Buffer.from("meta", "utf-8")];
        [pdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
            seeds,
            gatewayProgram.programId,
        );
        console.log("gateway pda account", pdaAccount.toString());
        pda_ata = await spl.getOrCreateAssociatedTokenAccount(
            conn,
            wallet,
            mint.publicKey,
            pdaAccount,
            true
        );
        console.log("pda_ata address", pda_ata.address.toString());
        const tx = new web3.Transaction();
        const memoInst = memo.createMemoInstruction(
            "this is a memo",
            [wallet.publicKey],
        );
        tx.add(memoInst);
        const depositInst = await gatewayProgram.methods.depositSplToken(
            new anchor.BN(1_000_000),
            Buffer.from("hello", "utf-8")).accounts(
            {
                from: tokenAccount.address,
                to: pda_ata.address,
            }
        ).instruction();
        tx.add(depositInst);
        const txsig = await anchor.web3.sendAndConfirmTransaction(conn, tx, [wallet]);


        try {
            await gatewayProgram.methods.depositSplToken(new anchor.BN(1_000_000), Buffer.from("world", "utf-8")).accounts(
                {
                    from: tokenAccount.address,
                    to: wallet_ata,
                }
            ).rpc();
            throw new Error("Expected error not thrown");
        } catch (err) {
            expect(err).to.be.instanceof(anchor.AnchorError);
            expect(err.message).to.include("DepositToAddressMismatch");
            // console.log("Error message: ", err.message);
        }
    });

    it("Withdraw 500_000 USDC from Gateway with ECDSA signature", async () => {

        // const tx_xfer = await spl.transfer(
        //     conn,
        //     wallet,
        //     tokenAccount.address,
        //     pda_ata.address,
        //     wallet,
        //     1_000_000
        // );
        // console.log("xfer tx hash", tx_xfer);
        const account2 = await spl.getAccount(conn, pda_ata.address);
        expect(account2.amount).to.be.eq(1_000_000n);
        // console.log("B4 withdraw: Account balance:", account2.amount.toString());


        const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
        console.log(`pda account data: nonce ${pdaAccountData.nonce}`);
        const hexAddr = bufferToHex(Buffer.from(pdaAccountData.tssAddress));
        console.log(`pda account data: tss address ${hexAddr}`);
        // const message_hash = fromHexString(
        //     "0a1e2723bd7f1996832b7ed7406df8ad975deba1aa04020b5bfc3e6fe70ecc29"
        // );
        // const signature = fromHexString(
        //     "58be181f57b2d56b0c252127c9874a8fbe5ebd04f7632fb3966935a3e9a765807813692cebcbf3416cb1053ad9c8c83af471ea828242cca22076dd04ddbcd253"
        // );
        const amount = new anchor.BN(500_000);
        const nonce = pdaAccountData.nonce;
        const buffer = Buffer.concat([
            nonce.toArrayLike(Buffer, 'be', 8),
            amount.toArrayLike(Buffer, 'be', 8),
            wallet_ata.toBuffer(),
        ]);
        const message_hash = keccak256(buffer);
        const signature = keyPair.sign(message_hash, 'hex');
        const { r, s, recoveryParam } = signature;
        const signatureBuffer = Buffer.concat([
            r.toArrayLike(Buffer, 'be', 32),
            s.toArrayLike(Buffer, 'be', 32),
        ]);

        await gatewayProgram.methods.withdrawSplToken(amount, Array.from(signatureBuffer), Number(recoveryParam), Array.from(message_hash), nonce)
            .accounts({
                from: pda_ata.address,
                to: wallet_ata,
            }).rpc();

        const account3 = await spl.getAccount(conn, pda_ata.address);
        expect(account3.amount).to.be.eq(500_000n);


        try {
            (await gatewayProgram.methods.withdrawSplToken(new anchor.BN(500_000), Array.from(signatureBuffer), Number(recoveryParam), Array.from(message_hash), nonce)
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

    it("deposit and withdraw 0.5 SOL from Gateway with ECDSA signature", async () => {
        await gatewayProgram.methods.deposit(new anchor.BN(1_000_000_000), Buffer.from("hello")).accounts({pda: pdaAccount}).rpc();
        // const transaction = new anchor.web3.Transaction();
        // transaction.add(
        //     web3.SystemProgram.transfer({
        //         fromPubkey: wallet.publicKey,
        //         toPubkey: pdaAccount,
        //         lamports: 1_000_000_000,
        //     })
        // );
        // await anchor.web3.sendAndConfirmTransaction(conn, transaction, [wallet]);
        let bal1 = await conn.getBalance(pdaAccount);
        console.log("pda account balance", bal1);
        expect(bal1).to.be.gte(1_000_000_000);

        const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
        console.log(`pda account data: nonce ${pdaAccountData.nonce}`);
        // const message_hash = fromHexString(
        //     "0a1e2723bd7f1996832b7ed7406df8ad975deba1aa04020b5bfc3e6fe70ecc29"
        // );
        // const signature = fromHexString(
        //     "58be181f57b2d56b0c252127c9874a8fbe5ebd04f7632fb3966935a3e9a765807813692cebcbf3416cb1053ad9c8c83af471ea828242cca22076dd04ddbcd253"
        // );
        const nonce = pdaAccountData.nonce;
        const amount = new anchor.BN(500000000);
        const to = wallet.publicKey;
        const buffer = Buffer.concat([
            nonce.toArrayLike(Buffer, 'be', 8),
            amount.toArrayLike(Buffer, 'be', 8),
            to.toBuffer(),
        ]);
        const message_hash = keccak256(buffer);
        const signature = keyPair.sign(message_hash, 'hex');
        const { r, s, recoveryParam } = signature;
        const signatureBuffer = Buffer.concat([
            r.toArrayLike(Buffer, 'be', 32),
            s.toArrayLike(Buffer, 'be', 32),
        ]);

        await gatewayProgram.methods.withdraw(
            amount, Array.from(signatureBuffer), Number(recoveryParam), Array.from(message_hash), nonce)
            .accounts({
                pda: pdaAccount,
                to: to,
            }).rpc();
        let bal2 = await conn.getBalance(pdaAccount);
        console.log("pda account balance", bal2);
        expect(bal2).to.be.eq(bal1 - 500_000_000);
        let bal3 = await conn.getBalance(to);
        expect(bal3).to.be.gte(500_000_000);
    })

    it("update TSS address", async () => {
        const newTss = new Uint8Array(20);
        randomFillSync(newTss);
        // console.log("generated new TSS address", newTss);
        await gatewayProgram.methods.updateTss(Array.from(newTss)).accounts({
            pda: pdaAccount,
        }).rpc();
        const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
        // console.log("updated TSS address", pdaAccountData.tssAddress);
        expect(pdaAccountData.tssAddress).to.be.deep.eq(Array.from(newTss));

        // only the authority stored in PDA can update the TSS address; the following should fail
        try {
            await gatewayProgram.methods.updateTss(Array.from(newTss)).accounts({
                pda: pdaAccount,
                signer: mint.publicKey,
            }).signers([mint]).rpc();
        } catch (err) {
            expect(err).to.be.instanceof(anchor.AnchorError);
            expect(err.message).to.include("SignerIsNotAuthority");
        }
    });
});


