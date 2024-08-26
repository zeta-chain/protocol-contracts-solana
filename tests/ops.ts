// this is for ops on devnet/mainnet
// uncomment Anchor.toml [test] to run this script
// #test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/ops.ts"

import * as anchor from "@coral-xyz/anchor";
import {Program, web3} from "@coral-xyz/anchor";
import {Gateway} from "../target/types/gateway";
import {expect} from "chai";
import {bufferToHex} from "ethereumjs-util";
import {getAccount} from "@solana/spl-token";

const programId = new web3.PublicKey("ZETAjseVjuFsxdRxo6MmTCvqFwb3ZHUx56Co3vCmGis");

(async ()=> {
    console.log("=====================================");
    console.log("BEGIN OPS ON DEVNET........");

    const provider = anchor.AnchorProvider.local("https://api.devnet.solana.com");
    anchor.setProvider(provider);
    console.log("wallet address:", provider.publicKey.toBase58());
    const conn = provider.connection;
    const bal = await conn.getBalance(provider.publicKey);
    console.log("balance:", bal);
    const wallet = anchor.workspace.Gateway.provider.wallet.payer;
    console.log("payer address:", wallet.publicKey.toBase58());

    const acctInfo = await conn.getAccountInfo(programId);
    console.log("acctInfo of gateway program :", acctInfo);

    const seeds = [Buffer.from("meta", "utf-8")];
    const [pdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
        seeds,
        programId,
    );
    console.log("pdaAccount:", pdaAccount.toBase58());

    const gatewayProgram = anchor.workspace.Gateway as Program<Gateway>;

    console.log("gateway provider", gatewayProgram.provider);
    const tssAddress = "0x8531a5ab847ff5b22d855633c25ed1da3255247e";
    // translate the above hex string into an array of number into tssAddress
    const tssAddressArray = tssAddress.slice(2).match(/.{1,2}/g).map(byte => parseInt(byte, 16));
    const chain_id = 901;
    const chain_id_bn = new anchor.BN(chain_id);

    console.log("tssAddressArray:", tssAddressArray);
    console.log("chain_id_bn:", chain_id_bn.toNumber());

    // Uncomment the following to initialize the gateway program;
    // can only be intialized once
    // try {
    //     const inst = await gatewayProgram.methods.initialize(tssAddressArray,chain_id_bn).transaction();
    //     console.log("initialize inst:", inst);
    //     const tx = await anchor.web3.sendAndConfirmTransaction(conn, inst, [wallet]);
    //     console.log("tx:", tx);
    // } catch (err) {
    //    console.log("intialize err:", err);
    // }

    const pdaAccountInfo = await conn.getAccountInfo(pdaAccount);
    console.log("pdaAccountInfoData:", pdaAccountInfo.data);
    // const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    try {
        console.log("fetching pda account data....", pdaAccount.toBase58());
        // const data = await gatewayProgram.account.pda.fetch(pdaAccount);
        const ainfo = await gatewayProgram.account.pda.fetch(pdaAccount)
        console.log(`pda account data: nonce ${ainfo.nonce}`);
        const hexAddr = bufferToHex(Buffer.from(ainfo.tssAddress));
        console.log(`pda account data: tss address ${hexAddr}`);
        console.log(`authority: ${ainfo.authority.toBase58()}`);
        console.log(`depositPaused: ${ainfo.depositPaused}`);
        console.log(`chain_id: ${ainfo.chainId}`);

        // console.log(`pda data:`, ainfo);
    } catch(e) {
        console.log("error:", e);
    }
    // console.log(`pda account data: nonce ${pdaAccountData.nonce}`);
    // const hexAddr = bufferToHex(Buffer.from(pdaAccountData.tssAddress));
    // console.log(`pda account data: tss address ${hexAddr}`);
    // console.log(`authority: ${pdaAccountData.authority.toBase58()}`);


    console.log("END OPS ON DEVNET........")
    console.log("=====================================");
})();




