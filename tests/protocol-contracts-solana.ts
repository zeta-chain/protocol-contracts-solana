import * as anchor from "@coral-xyz/anchor";
import {Program, web3} from "@coral-xyz/anchor";
import {Gateway} from "../target/types/gateway";
import * as spl from "@solana/spl-token";
import {randomFillSync} from 'crypto';
import { ec as EC } from 'elliptic';
import { keccak256 } from 'ethereumjs-util';
import { bufferToHex } from 'ethereumjs-util';
import {expect} from 'chai';
import {ecdsaRecover} from 'secp256k1';
import {getOrCreateAssociatedTokenAccount} from "@solana/spl-token";

console.log = function() {}

const ec = new EC('secp256k1');
// const keyPair = ec.genKeyPair();
// read private key from hex dump
const keyPair = ec.keyFromPrivate('5b81cdf52ba0766983acf8dd0072904733d92afe4dd3499e83e879b43ccb73e8');

const usdcDecimals = 6;

async function mintSPLToken(conn: anchor.web3.Connection, wallet: anchor.web3.Keypair, mint: anchor.web3.Keypair) {
    const mintRent = await spl.getMinimumBalanceForRentExemptMint(conn);
    let tokenTransaction = new anchor.web3.Transaction();
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
            usdcDecimals,
            wallet.publicKey,
            null,
        )
    );
    const txsig = await anchor.web3.sendAndConfirmTransaction(conn, tokenTransaction, [wallet, mint]);
    console.log("mint account created!", mint.publicKey.toString());
    return txsig;
}

async function depositSplTokens(gatewayProgram: Program<Gateway>, conn: anchor.web3.Connection, wallet: anchor.web3.Keypair, mint: anchor.web3.Keypair, address: Buffer) {
    let seeds = [Buffer.from("meta", "utf-8")];
    const [pdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
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

    let tokenAccount = await spl.getOrCreateAssociatedTokenAccount(
        conn,wallet, mint.publicKey, wallet.publicKey
    )
    await gatewayProgram.methods.depositSplToken(new anchor.BN(1_000_000), Array.from(address)).accounts({
        from: tokenAccount.address,
        to: pda_ata.address,
        mintAccount: mint.publicKey,
    }).rpc({commitment: 'processed'});
    return;
}

describe("some tests", () => {
    // Configure the client to use the local cluster.
    anchor.setProvider(anchor.AnchorProvider.env());
    const conn = anchor.getProvider().connection;
    const gatewayProgram = anchor.workspace.Gateway as Program<Gateway>;
    const wallet = anchor.workspace.Gateway.provider.wallet.payer;
    const mint = anchor.web3.Keypair.generate();
    const mint_fake = anchor.web3.Keypair.generate(); // for testing purpose


    let wallet_ata: anchor.web3.PublicKey;
    let pdaAccount: anchor.web3.PublicKey;

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
    // console.log("address", address);
    // const tssAddress = [239, 36, 74, 232, 12, 58, 220, 53, 101, 185, 127, 45, 0, 144, 15, 163, 104, 163, 74, 178,];
    const tssAddress = Array.from(address);
    console.log("tss address", tssAddress);

    const chain_id = 111111;
    const chain_id_bn = new anchor.BN(chain_id);

    let seeds = [Buffer.from("meta", "utf-8")];
    [pdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
        seeds,
        gatewayProgram.programId,
    );

    it("Initializes the program", async () => {
        await gatewayProgram.methods.initialize(tssAddress, chain_id_bn).rpc();

        // repeated initialization should fail
        try {
            await gatewayProgram.methods.initialize(tssAddress,chain_id_bn).rpc();
            throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
        } catch (err) {
            expect(err).to.be.not.null;
            // console.log("Error message: ", err.message)
        }
    });



    it("Mint a SPL USDC token", async () => {
        // now deploying a fake USDC SPL Token
        // 1. create a mint account
        await mintSPLToken(conn, wallet, mint);

        // 2. create token account to receive mint
        let tokenAccount = await spl.getOrCreateAssociatedTokenAccount(
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

        // create a fake USDC token account
        await mintSPLToken(conn, wallet, mint_fake);
        console.log("fake mint account created!", mint_fake.publicKey.toString());
    })

    it("whitelist USDC spl token", async () => {
        await gatewayProgram.methods.whitelistSplMint().accounts({
            whitelistCandidate: mint.publicKey,
        }).signers([]).rpc();

        let seeds = [Buffer.from("whitelist", "utf-8"), mint.publicKey.toBuffer()];
        let [entryAddress] = anchor.web3.PublicKey.findProgramAddressSync(
            seeds,
            gatewayProgram.programId,
        );
        let entry = await gatewayProgram.account.whitelistEntry.fetch(entryAddress)
        console.log("whitelist entry", entry);

        try {
            seeds = [Buffer.from("whitelist", "utf-8"), mint_fake.publicKey.toBuffer()];
            [entryAddress] = anchor.web3.PublicKey.findProgramAddressSync(
                seeds,
                gatewayProgram.programId,
            );
            entry = await gatewayProgram.account.whitelistEntry.fetch(entryAddress);
            console.log("whitelist entry", entry);
        } catch(err) {
            expect(err.message).to.include("Account does not exist or has no data");
        }
    });

    it("Deposit 1_000_000 USDC to Gateway", async () => {
        let pda_ata = await getOrCreateAssociatedTokenAccount(conn, wallet, mint.publicKey, pdaAccount, true);
        let acct = await spl.getAccount(conn, pda_ata.address);
        let bal0 = acct.amount;
        await depositSplTokens(gatewayProgram, conn, wallet, mint, address);
        acct = await spl.getAccount(conn, pda_ata.address);
        let bal1 = acct.amount;
        expect(bal1-bal0).to.be.eq(1_000_000n);


        let tokenAccount = await getOrCreateAssociatedTokenAccount(
            conn,wallet, mint.publicKey, wallet.publicKey,
        )
        try {
            await gatewayProgram.methods.depositSplToken(new anchor.BN(1_000_000), Array.from(address)).accounts(
                {
                    from: tokenAccount.address,
                    to: wallet_ata,
                    mintAccount: mint.publicKey,
                }
            ).rpc();
            throw new Error("Expected error not thrown");
        } catch (err) {
            expect(err).to.be.instanceof(anchor.AnchorError);
            expect(err.message).to.include("DepositToAddressMismatch");
            // console.log("Error message: ", err.message);
        }

        // test depositSplTokenAndCall
        acct = await spl.getAccount(conn, pda_ata.address);
        bal0 = acct.amount;
        await gatewayProgram.methods.depositSplTokenAndCall(new anchor.BN(2_000_000), Array.from(address), Buffer.from('hi', 'utf-8')).accounts({
            from: tokenAccount.address,
            to: pda_ata.address,
            mintAccount: mint.publicKey,
        }).rpc({commitment: 'processed'});
        acct = await spl.getAccount(conn, pda_ata.address);
        bal1 = acct.amount;
        expect(bal1-bal0).to.be.eq(2_000_000n);
    });

    it("deposit non-whitelisted SPL tokens should fail", async () => {
        let seeds = [Buffer.from("meta", "utf-8")];
        [pdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
            seeds,
            gatewayProgram.programId,
        );
        console.log("gateway pda account", pdaAccount.toString());
        let fake_pda_ata = await spl.getOrCreateAssociatedTokenAccount(
            conn,
            wallet,
            mint_fake.publicKey,
            pdaAccount,
            true
        );
        console.log("fake_mint fake_pda_ata address", fake_pda_ata.address.toString());

        let tokenAccount = await spl.getOrCreateAssociatedTokenAccount(
            conn,wallet, mint_fake.publicKey, wallet.publicKey, true
        )
        try {
            await gatewayProgram.methods.depositSplToken(new anchor.BN(1_000_000), Array.from(address)).accounts({
                from: tokenAccount.address,
                to: fake_pda_ata.address,
                mintAccount: mint_fake.publicKey,
            }).rpc({commitment: 'processed'});
        } catch (err) {
            expect(err).to.be.instanceof(anchor.AnchorError);
            expect(err.message).to.include("AccountNotInitialized");
        }

    });

    it("Withdraw 500_000 USDC from Gateway with ECDSA signature", async () => {
        let pda_ata = await spl.getAssociatedTokenAddress(mint.publicKey, pdaAccount, true);

        const account2 = await spl.getAccount(conn, pda_ata);
        // expect(account2.amount).to.be.eq(1_000_000n);
        console.log("B4 withdraw: Account balance:", account2.amount.toString());


        const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
        console.log(`pda account data: nonce ${pdaAccountData.nonce}`);
        const hexAddr = bufferToHex(Buffer.from(pdaAccountData.tssAddress));
        console.log(`pda account data: tss address ${hexAddr}`);

        const amount = new anchor.BN(500_000);
        const nonce = pdaAccountData.nonce;
        const buffer = Buffer.concat([
            Buffer.from("withdraw_spl_token","utf-8"),
            chain_id_bn.toArrayLike(Buffer, 'be', 8),
            nonce.toArrayLike(Buffer, 'be', 8),
            amount.toArrayLike(Buffer, 'be', 8),
            mint.publicKey.toBuffer(),
            wallet_ata.toBuffer(),
        ]);
        const message_hash = keccak256(buffer);
        const signature = keyPair.sign(message_hash, 'hex');
        const { r, s, recoveryParam } = signature;
        const signatureBuffer = Buffer.concat([
            r.toArrayLike(Buffer, 'be', 32),
            s.toArrayLike(Buffer, 'be', 32),
        ]);

        await gatewayProgram.methods.withdrawSplToken(usdcDecimals,amount, Array.from(signatureBuffer), Number(recoveryParam), Array.from(message_hash), nonce)
            .accounts({
                pdaAta: pda_ata,
                mintAccount: mint.publicKey,
                to: wallet_ata,
            }).rpc();

        const account3 = await spl.getAccount(conn, pda_ata);
        expect(account3.amount-account2.amount).to.be.eq(-500_000n);


        try {
            (await gatewayProgram.methods.withdrawSplToken(usdcDecimals,new anchor.BN(500_000), Array.from(signatureBuffer), Number(recoveryParam), Array.from(message_hash), nonce)
                .accounts({
                    pdaAta: pda_ata,
                    mintAccount: mint.publicKey,
                    to: wallet_ata,
                }).rpc());
            throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
        } catch (err) {
            expect(err).to.be.instanceof(anchor.AnchorError);
            expect(err.message).to.include("NonceMismatch");
            const account4 = await spl.getAccount(conn, pda_ata);
            console.log("After 2nd withdraw: Account balance:", account4.amount.toString());
            expect(account4.amount).to.be.eq(2_500_000n);
        }


        try {
            const nonce2 = nonce.addn(1)
            const buffer = Buffer.concat([
                Buffer.from("withdraw_spl_token","utf-8"),
                chain_id_bn.toArrayLike(Buffer, 'be', 8),
                nonce2.toArrayLike(Buffer, 'be', 8),
                amount.toArrayLike(Buffer, 'be', 8),
                mint_fake.publicKey.toBuffer(),
                wallet_ata.toBuffer(),
            ]);
            const message_hash = keccak256(buffer);
            const signature = keyPair.sign(message_hash, 'hex');
            const { r, s, recoveryParam } = signature;
            const signatureBuffer = Buffer.concat([
                r.toArrayLike(Buffer, 'be', 32),
                s.toArrayLike(Buffer, 'be', 32),
            ]);
            await gatewayProgram.methods.withdrawSplToken(usdcDecimals,amount, Array.from(signatureBuffer), Number(recoveryParam), Array.from(message_hash), nonce2 )
                .accounts({
                    pdaAta: pda_ata,
                    mintAccount: mint_fake.publicKey,
                    to: wallet_ata,
                }).rpc();
            throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
        } catch (err) {
            expect(err).to.be.instanceof(anchor.AnchorError);
            console.log("Error message: ", err.message);
            expect(err.message).to.include("ConstraintTokenMint");
            const account4 = await spl.getAccount(conn, pda_ata);
            console.log("After 2nd withdraw: Account balance:", account4.amount.toString());
            expect(account4.amount).to.be.eq(2_500_000n);
        }

    });

    it("deposit and withdraw 0.5 SOL from Gateway with ECDSA signature", async () => {
        await gatewayProgram.methods.deposit(new anchor.BN(1_000_000_000), Array.from(address)).accounts({}).rpc();
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
        const to = await spl.getAssociatedTokenAddress(mint.publicKey, wallet.publicKey);
        const buffer = Buffer.concat([
            Buffer.from("withdraw","utf-8"),
            chain_id_bn.toArrayLike(Buffer, 'be', 8),
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
                to: to,
            }).rpc();
        let bal2 = await conn.getBalance(pdaAccount);
        console.log("pda account balance", bal2);
        expect(bal2).to.be.eq(bal1 - 500_000_000);
        let bal3 = await conn.getBalance(to);
        expect(bal3).to.be.gte(500_000_000);
    })

    it("deposit and call", async () => {
        let bal1 = await conn.getBalance(pdaAccount);
        const txsig = await gatewayProgram.methods.depositAndCall(new anchor.BN(1_000_000_000), Array.from(address), Buffer.from("hello", "utf-8")).accounts({}).rpc({commitment: 'processed'});
        const tx =  await conn.getParsedTransaction(txsig, 'confirmed');
        console.log("deposit and call parsed tx", tx);
        let bal2 = await conn.getBalance(pdaAccount);
        expect(bal2-bal1).to.be.gte(1_000_000_000);
    })

    it("unwhitelist SPL token and deposit should fail", async () => {
        await gatewayProgram.methods.unwhitelistSplMint().accounts({
            whitelistCandidate: mint.publicKey,
        }).rpc();

        try {
            await depositSplTokens(gatewayProgram, conn, wallet, mint, address)
        } catch (err) {
            expect(err).to.be.instanceof(anchor.AnchorError);
            expect(err.message).to.include("AccountNotInitialized");
        }
    });

    it("re-whitelist SPL token and deposit should succeed", async () => {
        await gatewayProgram.methods.whitelistSplMint().accounts({
            whitelistCandidate: mint.publicKey,
        }).rpc();
        await depositSplTokens(gatewayProgram, conn, wallet, mint, address);
    });


    it("update TSS address", async () => {
        const newTss = new Uint8Array(20);
        randomFillSync(newTss);
        // console.log("generated new TSS address", newTss);
        await gatewayProgram.methods.updateTss(Array.from(newTss)).accounts({
            // pda: pdaAccount,
        }).rpc();
        const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
        // console.log("updated TSS address", pdaAccountData.tssAddress);
        expect(pdaAccountData.tssAddress).to.be.deep.eq(Array.from(newTss));

        // only the authority stored in PDA can update the TSS address; the following should fail
        try {
            await gatewayProgram.methods.updateTss(Array.from(newTss)).accounts({
                signer: mint.publicKey,
            }).signers([mint]).rpc();
        } catch (err) {
            expect(err).to.be.instanceof(anchor.AnchorError);
            expect(err.message).to.include("SignerIsNotAuthority");
        }
    });

    it("pause deposit and deposit should fail", async () => {
        const newTss = new Uint8Array(20);
        randomFillSync(newTss);
        // console.log("generated new TSS address", newTss);
        await gatewayProgram.methods.setDepositPaused(true).accounts({

        }).rpc();

        // now try deposit, should fail
        try {
            await gatewayProgram.methods.depositAndCall(new anchor.BN(1_000_000), Array.from(address), Buffer.from('hi', 'utf-8')).accounts({}).rpc();
        } catch (err) {
            console.log("Error message: ", err.message);
            expect(err).to.be.instanceof(anchor.AnchorError);
            expect(err.message).to.include("DepositPaused");
        }
    });




    const newAuthority = anchor.web3.Keypair.generate();
    it("update authority", async () => {
        await gatewayProgram.methods.updateAuthority(newAuthority.publicKey).accounts({

        }).rpc();
        // const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
        // expect(pdaAccountData.authority).to.be.eq(newAuthority.publicKey);

        // now the old authority cannot update TSS address and will fail
        try {
            await gatewayProgram.methods.updateTss(Array.from(new Uint8Array(20))).accounts({

            }).rpc();
        } catch (err) {
            console.log("Error message: ", err.message);
            expect(err).to.be.instanceof(anchor.AnchorError);
            expect(err.message).to.include("SignerIsNotAuthority");
        }
    });

});





