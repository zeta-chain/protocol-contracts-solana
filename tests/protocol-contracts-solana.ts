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



const ec = new EC('secp256k1');
// const keyPair = ec.genKeyPair();
// read private key from hex dump
const keyPair = ec.keyFromPrivate('5b81cdf52ba0766983acf8dd0072904733d92afe4dd3499e83e879b43ccb73e8');

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


    it("deposit and withdraw 0.5 SOL from Gateway with ECDSA signature", async () => {
        await gatewayProgram.methods.deposit(new anchor.BN(1_000_000_000), Array.from(address)).accounts({pda: pdaAccount}).rpc();
        let bal1 = await conn.getBalance(pdaAccount);
        console.log("pda account balance", bal1);
        expect(bal1).to.be.gte(1_000_000_000);

        const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
        console.log(`pda account data: nonce ${pdaAccountData.nonce}`);
        const nonce = pdaAccountData.nonce;
        const amount = new anchor.BN(500000000);
        const to = wallet.publicKey;
        const buffer = Buffer.concat([
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
                pda: pdaAccount,
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
        await gatewayProgram.methods.depositAndCall(new anchor.BN(1_000_000_000), Array.from(address), Buffer.from("hello", "utf-8")).accounts({pda: pdaAccount}).rpc();
        let bal2 = await conn.getBalance(pdaAccount);
        expect(bal2-bal1).to.be.gte(1_000_000_000);
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

    it("pause deposit and deposit should fail", async () => {
        const newTss = new Uint8Array(20);
        randomFillSync(newTss);
        // console.log("generated new TSS address", newTss);
        await gatewayProgram.methods.setDepositPaused(true).accounts({
            pda: pdaAccount,
        }).rpc();

        // now try deposit, should fail
        try {
            await gatewayProgram.methods.deposit(new anchor.BN(1_000_000), Array.from(address)).accounts({pda: pdaAccount}).rpc();
        } catch (err) {
            console.log("Error message: ", err.message);
            expect(err).to.be.instanceof(anchor.AnchorError);
            expect(err.message).to.include("DepositPaused");
        }

    });

    it("update authority", async () => {
        const newAuthority = anchor.web3.Keypair.generate();
        await gatewayProgram.methods.updateAuthority(newAuthority.publicKey).accounts({
            pda: pdaAccount,
        }).rpc();
        // const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
        // expect(pdaAccountData.authority).to.be.eq(newAuthority.publicKey);

        // now the old authority cannot update TSS address and will fail
        try {
            await gatewayProgram.methods.updateTss(Array.from(new Uint8Array(20))).accounts({
                pda: pdaAccount,
            }).rpc();
        } catch (err) {
            console.log("Error message: ", err.message);
            expect(err).to.be.instanceof(anchor.AnchorError);
            expect(err.message).to.include("SignerIsNotAuthority");
        }
    });


});


