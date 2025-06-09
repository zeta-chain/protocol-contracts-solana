// this is for ops on devnet/mainnet
// uncomment Anchor.toml [test] to run this script
// #test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/ops.ts"

import * as anchor from "@coral-xyz/anchor";
import { Program, web3 } from "@coral-xyz/anchor";
import { Gateway } from "../target/types/gateway";
import { expect } from "chai";
import { bufferToHex } from "ethereumjs-util";
import { getAccount, transferInstructionData } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import Squads, { DEFAULT_MULTISIG_PROGRAM_ID } from "@sqds/sdk";
import bs58 from "bs58";

const programId = new web3.PublicKey(
  "ZETAjseVjuFsxdRxo6MmTCvqFwb3ZHUx56Co3vCmGis"
);

(async () => {
  console.log("=====================================");
  console.log("BEGIN OPS ON DEVNET........");
  const provider = anchor.AnchorProvider.local(
    "https://api.mainnet-beta.solana.com"
  );
  anchor.setProvider(provider);
  console.log("wallet address:", provider.publicKey.toBase58());
  const conn = provider.connection;
  const bal = await conn.getBalance(provider.publicKey);
  console.log("balance:", bal);
  const wallet = provider.wallet;
  console.log("payer address:", wallet.publicKey.toBase58());
  const gatewayProgram = anchor.workspace.Gateway as Program<Gateway>;
  console.log("program ", gatewayProgram.programId.toString());
  let gatewayAuthority;
  try {
    const seeds = [Buffer.from("meta", "utf-8")];
    const [pdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      gatewayProgram.programId
    );
    console.log("pdaAccount:", pdaAccount.toBase58());
    const ainfo = await gatewayProgram.account.pda.fetch(pdaAccount);
    console.log(`  nonce ${ainfo.nonce}`);
    const hexAddr = bufferToHex(Buffer.from(ainfo.tssAddress));
    console.log(`  tss address ${hexAddr}`);
    console.log(`  authority: ${ainfo.authority.toBase58()}`);
    gatewayAuthority = ainfo.authority;
    console.log(`  depositPaused: ${ainfo.depositPaused}`);
    console.log(`  chain_id: ${ainfo.chainId}`);
  } catch (e) {
    console.log("exception", e);
  }
  // return;
  {
    try {
      const signer = anchor.workspace.Gateway.provider.wallet.payer;

      // const txsig = await gatewayProgram.methods
      //   .setDepositPaused(false)
      //   .accounts({
      //     signer: wallet.publicKey,
      //   })
      //   .signers([signer])
      //   .rpc();
      // console.log("txsign", txsig);
      //
      const new_authority = new PublicKey(
        "AuMwEjGF4K7JcfbdyuWYiPe1V9wQajeBwC7bNUoLRJ9o"
      );
      console.log("transfering authority to ", new_authority.toString());

      const txsig = await gatewayProgram.methods
        .updateAuthority(new_authority)
        .accounts({
          signer: wallet.publicKey,
        })
        .signers([signer])
        .rpc();

      console.log("tx", txsig);
    } catch (e) {
      console.log("exception:", e);
    }
  }
})();
