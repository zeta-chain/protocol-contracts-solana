// this is for ops on devnet/mainnet
// uncomment Anchor.toml [test] to run this script
// #test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/ops.ts"

import * as anchor from "@coral-xyz/anchor";
import { Program, web3 } from "@coral-xyz/anchor";
import { Gateway } from "../target/types/gateway";
import { expect } from "chai";
import { bufferToHex } from "ethereumjs-util";
import { getAccount } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import Squads, { DEFAULT_MULTISIG_PROGRAM_ID } from "@sqds/sdk";

const programId = new web3.PublicKey(
  "ZETAjseVjuFsxdRxo6MmTCvqFwb3ZHUx56Co3vCmGis"
);

(async () => {
  console.log("=====================================");
  console.log("BEGIN OPS ON DEVNET........");
  const provider = anchor.AnchorProvider.local("https://api.devnet.solana.com");
  anchor.setProvider(provider);
  console.log("wallet address:", provider.publicKey.toBase58());
  const conn = provider.connection;
  const bal = await conn.getBalance(provider.publicKey);
  console.log("balance:", bal);
  const wallet = provider.wallet;
  console.log("payer address:", wallet.publicKey.toBase58());
  const gatewayProgram = anchor.workspace.Gateway as Program<Gateway>;
  console.log("program ", gatewayProgram.programId.toString());
  try {
    const seeds = [Buffer.from("meta", "utf-8")];
    const [pdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      gatewayProgram.programId
    );
    console.log("pdaAccount:", pdaAccount.toBase58());
    const ainfo = await gatewayProgram.account.pda.fetch(pdaAccount);
    console.log(`pda account data: nonce ${ainfo.nonce}`);
    const hexAddr = bufferToHex(Buffer.from(ainfo.tssAddress));
    console.log(`pda account data: tss address ${hexAddr}`);
    console.log(`authority: ${ainfo.authority.toBase58()}`);
    console.log(`depositPaused: ${ainfo.depositPaused}`);
    console.log(`chain_id: ${ainfo.chainId}`);
  } catch (e) {
    console.log("exception", e);
  }
  // return;
  {
    console.log("Squads v3 multisig info");
    const multisigPK = new PublicKey(
      "5tM6ytRSy2wXFg6bFK4xZXpHRCyjduNHqSxYSPQqqzGd"
    );
    const squads = Squads.devnet(provider.wallet);
    console.log("program id", DEFAULT_MULTISIG_PROGRAM_ID.toString());
    const ms = await squads.getMultisig(multisigPK);
    console.log("ms", ms.publicKey.toString());
    console.log("ms authorityIndex", ms.authorityIndex);
    console.log("ms transactionIndex", ms.transactionIndex);
    const authorityPDA = squads.getAuthorityPDA(ms.publicKey, 1);
    console.log("authorityPDA", authorityPDA.toString());
    try {
      const updateAuthorityInst = await gatewayProgram.methods
        .updateAuthority(wallet.publicKey)
        .accounts({
          signer: authorityPDA,
        })
        .instruction();
      console.log("updateAuthorityInst", updateAuthorityInst);
      const msTx = await squads.createTransaction(multisigPK, 1);
      console.log("create tx: ", msTx.publicKey);
      const ixRes = await squads.addInstruction(
        msTx.publicKey,
        updateAuthorityInst
      );
      console.log("addInstruction result", ixRes);
      await squads.activateTransaction(msTx.publicKey);
    } catch (e) {
      console.log("exception:", e);
    }
  }
})();
