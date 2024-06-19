import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Gateway } from "../target/types/gateway";
import * as spl from "@solana/spl-token";

describe("some tests", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const conn = anchor.getProvider().connection;

  const program = anchor.workspace.Gateway as Program<Gateway>;

  it("Test suite 1", async () => {
    let transaction = new anchor.web3.Transaction();
    const wallet = anchor.workspace.Gateway.provider.wallet.payer;
    console.log("wallet address", wallet.publicKey.toString());
    // Add your test here.
    transaction.add(await  program.methods.initialize().instruction());
    const tx = await anchor.web3.sendAndConfirmTransaction(anchor.getProvider().connection, transaction,[wallet]);
    // console.log("Your transaction signature", tx);

    // let txres = await anchor.getProvider().connection.getParsedTransaction(tx, {commitment:"confirmed"});
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
    await anchor.web3.sendAndConfirmTransaction(conn, tokenTransaction,[wallet, mint]);
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
            1_000_000,
        )
    );
    await anchor.web3.sendAndConfirmTransaction(anchor.getProvider().connection, mintToTransaction, [wallet]);
    console.log("Minted 10 USDC to:", tokenAccount.address.toString());
    const account = await spl.getAccount(conn, tokenAccount.address);
    console.log("Account balance:", account.amount.toString());
    console.log("Account owner: ", account.owner.toString());



  });


});
