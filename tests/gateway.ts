import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Gateway } from "../target/types/gateway";
import * as spl from "@solana/spl-token";
import { randomFillSync } from "crypto";
import { ec as EC } from "elliptic";
import { keccak256 } from "ethereumjs-util";
import { expect } from "chai";
import { getOrCreateAssociatedTokenAccount } from "@solana/spl-token";
import { Connected } from "../target/types/connected";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import { ConnectedSpl } from "../target/types/connected_spl";
import { ComputeBudgetProgram } from "@solana/web3.js";

const ec = new EC("secp256k1");
// read private key from hex dump
const keyPair = ec.keyFromPrivate(
  "5b81cdf52ba0766983acf8dd0072904733d92afe4dd3499e83e879b43ccb73e8"
);

const publicKeyBuffer = Buffer.from(
  keyPair.getPublic(false, "hex").slice(2),
  "hex"
); // Uncompressed form of public key, remove the '04' prefix

const addressBuffer = keccak256(publicKeyBuffer); // Skip the first byte (format indicator)
const address = addressBuffer.slice(-20);
const tssAddress = Array.from(address);

// generic revertOptions
const revertOptions = {
  revertAddress: anchor.web3.Keypair.generate().publicKey,
  abortAddress: tssAddress, // using tss address for simplicity, used just on protocol side
  callOnRevert: false,
  revertMessage: Buffer.from("", "utf-8"),
  onRevertGasLimit: new anchor.BN(0),
};

const usdcDecimals = 6;
const chain_id = 111111;
const chain_id_bn = new anchor.BN(chain_id);
const maxPayloadSize = 745;

async function mintSPLToken(
  conn: anchor.web3.Connection,
  wallet: anchor.web3.Keypair,
  mint: anchor.web3.Keypair
) {
  const mintRent = await spl.getMinimumBalanceForRentExemptMint(conn);
  let tokenTransaction = new anchor.web3.Transaction();
  tokenTransaction.add(
    anchor.web3.SystemProgram.createAccount({
      fromPubkey: wallet.publicKey,
      newAccountPubkey: mint.publicKey,
      lamports: mintRent,
      space: spl.MINT_SIZE,
      programId: spl.TOKEN_PROGRAM_ID,
    }),
    spl.createInitializeMintInstruction(
      mint.publicKey,
      usdcDecimals,
      wallet.publicKey,
      null
    )
  );
  const txsig = await anchor.web3.sendAndConfirmTransaction(
    conn,
    tokenTransaction,
    [wallet, mint]
  );
  return txsig;
}

async function depositSplTokens(
  gatewayProgram: Program<Gateway>,
  conn: anchor.web3.Connection,
  wallet: anchor.web3.Keypair,
  mint: anchor.web3.Keypair,
  address: Buffer
) {
  let seeds = [Buffer.from("meta", "utf-8")];
  const [pdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
    seeds,
    gatewayProgram.programId
  );
  const pda_ata = await spl.getOrCreateAssociatedTokenAccount(
    conn,
    wallet,
    mint.publicKey,
    pdaAccount,
    true
  );

  let tokenAccount = await spl.getOrCreateAssociatedTokenAccount(
    conn,
    wallet,
    mint.publicKey,
    wallet.publicKey
  );
  await gatewayProgram.methods
    .depositSplToken(
      new anchor.BN(1_000_000),
      Array.from(address),
      revertOptions
    )
    .accounts({
      from: tokenAccount.address,
      to: pda_ata.address,
      mintAccount: mint.publicKey,
    })
    .rpc({ commitment: "processed" });
  return;
}

async function withdrawSplToken(
  mint,
  decimals,
  amount,
  nonce,
  from,
  to,
  to_owner,
  gatewayProgram: Program<Gateway>
) {
  const buffer = Buffer.concat([
    Buffer.from("ZETACHAIN", "utf-8"),
    Buffer.from([0x02]),
    chain_id_bn.toArrayLike(Buffer, "be", 8),
    nonce.toArrayLike(Buffer, "be", 8),
    amount.toArrayLike(Buffer, "be", 8),
    mint.publicKey.toBuffer(),
    to.toBuffer(),
  ]);
  const message_hash = keccak256(buffer);
  const signature = keyPair.sign(message_hash, "hex");
  const { r, s, recoveryParam } = signature;
  const signatureBuffer = Buffer.concat([
    r.toArrayLike(Buffer, "be", 32),
    s.toArrayLike(Buffer, "be", 32),
  ]);
  return gatewayProgram.methods
    .withdrawSplToken(
      decimals,
      amount,
      Array.from(signatureBuffer),
      Number(recoveryParam),
      Array.from(message_hash),
      nonce
    )
    .accounts({
      pdaAta: from,
      mintAccount: mint.publicKey,
      recipientAta: to,
      recipient: to_owner,
    })
    .rpc({ commitment: "processed" });
}

describe("Gateway", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const conn = anchor.getProvider().connection;
  const gatewayProgram = anchor.workspace.Gateway as Program<Gateway>;
  const connectedProgram = anchor.workspace.Connected as Program<Connected>;
  const connectedSPLProgram = anchor.workspace
    .ConnectedSPL as Program<ConnectedSpl>;
  const wallet = anchor.workspace.Gateway.provider.wallet.payer;
  const mint = anchor.web3.Keypair.generate();
  const mint_fake = anchor.web3.Keypair.generate(); // for testing purpose
  const random_account = anchor.web3.Keypair.generate();

  let wallet_ata: anchor.web3.PublicKey;
  let pdaAccount: anchor.web3.PublicKey;

  let seeds = [Buffer.from("meta", "utf-8")];
  [pdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
    seeds,
    gatewayProgram.programId
  );

  it("Initializes the program", async () => {
    await gatewayProgram.methods.initialize(tssAddress, chain_id_bn).rpc();

    // repeated initialization should fail
    try {
      await gatewayProgram.methods.initialize(tssAddress, chain_id_bn).rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.not.null;
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
      wallet.publicKey
    );
    // 3. mint some tokens
    const mintToTransaction = new anchor.web3.Transaction().add(
      spl.createMintToInstruction(
        mint.publicKey,
        tokenAccount.address,
        wallet.publicKey,
        10_000_000
      )
    );
    await anchor.web3.sendAndConfirmTransaction(
      anchor.getProvider().connection,
      mintToTransaction,
      [wallet]
    );

    // OK; transfer some USDC SPL token to the gateway PDA
    wallet_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      wallet.publicKey
    );

    // create a fake USDC token account
    await mintSPLToken(conn, wallet, mint_fake);

    // 2. create token account to receive mint
    let fakeMintTokenAccount = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint_fake.publicKey,
      wallet.publicKey
    );
    // 3. mint some tokens
    const mintFakeToTransaction = new anchor.web3.Transaction().add(
      spl.createMintToInstruction(
        mint_fake.publicKey,
        fakeMintTokenAccount.address,
        wallet.publicKey,
        10_000_000
      )
    );
    await anchor.web3.sendAndConfirmTransaction(
      anchor.getProvider().connection,
      mintFakeToTransaction,
      [wallet]
    );
  });

  it("Whitelist USDC SPL token", async () => {
    await gatewayProgram.methods
      .whitelistSplMint([], 0, [], new anchor.BN(0))
      .accounts({
        whitelistCandidate: mint.publicKey,
      })
      .signers([])
      .rpc();

    let seeds = [Buffer.from("whitelist", "utf-8"), mint.publicKey.toBuffer()];
    let [entryAddress] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      gatewayProgram.programId
    );

    try {
      seeds = [
        Buffer.from("whitelist", "utf-8"),
        mint_fake.publicKey.toBuffer(),
      ];
      [entryAddress] = anchor.web3.PublicKey.findProgramAddressSync(
        seeds,
        gatewayProgram.programId
      );
      await gatewayProgram.account.whitelistEntry.fetch(entryAddress);
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err.message).to.include("Account does not exist or has no data");
    }
  });

  it("Deposit 1_000_000 USDC with above max payload size should fail", async () => {
    const pda_ata = await getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      pdaAccount,
      true
    );
    const tokenAccount = await getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      wallet.publicKey
    );
    try {
      await gatewayProgram.methods
        .depositSplTokenAndCall(
          new anchor.BN(2_000_000),
          Array.from(address),
          Buffer.from(Array(maxPayloadSize + 1).fill(1)),
          null
        )
        .accounts({
          from: tokenAccount.address,
          to: pda_ata.address,
          mintAccount: mint.publicKey,
        })
        .preInstructions([
          ComputeBudgetProgram.setComputeUnitLimit({ units: 400000 }),
        ])
        .rpc({ commitment: "processed" });
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("MemoLengthExceeded");
    }
  });

  it("Deposit 1_000_000 USDC with with max payload size", async () => {
    const pda_ata = await getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      pdaAccount,
      true
    );
    const tokenAccount = await getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      wallet.publicKey
    );
    let acct = await spl.getAccount(conn, pda_ata.address);
    const bal1 = acct.amount;

    await gatewayProgram.methods
      .depositSplTokenAndCall(
        new anchor.BN(2_000_000),
        Array.from(address),
        Buffer.from(Array(maxPayloadSize).fill(1)),
        null
      )
      .accounts({
        from: tokenAccount.address,
        to: pda_ata.address,
        mintAccount: mint.publicKey,
      })
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 400000 }),
      ])
      .rpc({ commitment: "processed" });
    acct = await spl.getAccount(conn, pda_ata.address);
    const bal2 = acct.amount;
    expect(bal2 - bal1).to.be.eq(2_000_000n);
  });

  it("Deposit 1_000_000 USDC to Gateway", async () => {
    let pda_ata = await getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      pdaAccount,
      true
    );
    let acct = await spl.getAccount(conn, pda_ata.address);
    let bal0 = acct.amount;
    await depositSplTokens(gatewayProgram, conn, wallet, mint, address);
    acct = await spl.getAccount(conn, pda_ata.address);
    let bal1 = acct.amount;
    expect(bal1 - bal0).to.be.eq(1_000_000n);

    let tokenAccount = await getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      wallet.publicKey
    );
    try {
      await gatewayProgram.methods
        .depositSplToken(
          new anchor.BN(1_000_000),
          Array.from(address),
          revertOptions
        )
        .accounts({
          from: tokenAccount.address,
          to: wallet_ata,
          mintAccount: mint.publicKey,
        })
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("DepositToAddressMismatch");
    }

    // test depositSplTokenAndCall
    acct = await spl.getAccount(conn, pda_ata.address);
    bal0 = acct.amount;
    await gatewayProgram.methods
      .depositSplTokenAndCall(
        new anchor.BN(2_000_000),
        Array.from(address),
        Buffer.from("hi", "utf-8"),
        revertOptions
      )
      .accounts({
        from: tokenAccount.address,
        to: pda_ata.address,
        mintAccount: mint.publicKey,
      })
      .rpc({ commitment: "processed" });
    acct = await spl.getAccount(conn, pda_ata.address);
    bal1 = acct.amount;
    expect(bal1 - bal0).to.be.eq(2_000_000n);
  });

  it("Deposit 1_000_000 fake spl to Gateway fails", async () => {
    let fake_pda_ata = await getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint_fake.publicKey,
      pdaAccount,
      true
    );

    let fake_tokenAccount = await getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint_fake.publicKey,
      wallet.publicKey,
      true
    );
    try {
      await gatewayProgram.methods
        .depositSplToken(
          new anchor.BN(1_000_000),
          Array.from(address),
          revertOptions
        )
        .accounts({
          from: fake_tokenAccount.address,
          to: fake_pda_ata.address,
          mintAccount: mint.publicKey,
        })
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("ConstraintRaw.");
    }
  });

  it("Deposit non-whitelisted SPL tokens should fail", async () => {
    let seeds = [Buffer.from("meta", "utf-8")];
    [pdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      gatewayProgram.programId
    );
    let fake_pda_ata = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint_fake.publicKey,
      pdaAccount,
      true
    );

    let tokenAccount = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint_fake.publicKey,
      wallet.publicKey,
      true
    );
    try {
      await gatewayProgram.methods
        .depositSplToken(
          new anchor.BN(1_000_000),
          Array.from(address),
          revertOptions
        )
        .accounts({
          from: tokenAccount.address,
          to: fake_pda_ata.address,
          mintAccount: mint_fake.publicKey,
        })
        .rpc({ commitment: "processed" });
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("AccountNotInitialized");
    }
  });

  it("Withdraw 500_000 USDC from Gateway with ECDSA signature", async () => {
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const account2 = await spl.getAccount(conn, pda_ata);

    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;
    await withdrawSplToken(
      mint,
      usdcDecimals,
      amount,
      nonce,
      pda_ata,
      wallet_ata,
      wallet.publicKey,
      gatewayProgram
    );
    const account3 = await spl.getAccount(conn, pda_ata);
    expect(account3.amount - account2.amount).to.be.eq(-500_000n);

    // should trigger nonce mismatch in withdraw
    try {
      await withdrawSplToken(
        mint,
        usdcDecimals,
        amount,
        nonce,
        pda_ata,
        wallet_ata,
        wallet.publicKey,
        gatewayProgram
      );
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("NonceMismatch");
      const account4 = await spl.getAccount(conn, pda_ata);
      expect(account4.amount).to.be.eq(4_500_000n);
    }

    try {
      const nonce2 = nonce.addn(1);
      const buffer = Buffer.concat([
        Buffer.from("ZETACHAIN", "utf-8"),
        Buffer.from([0x02]),
        chain_id_bn.toArrayLike(Buffer, "be", 8),
        nonce2.toArrayLike(Buffer, "be", 8),
        amount.toArrayLike(Buffer, "be", 8),
        mint_fake.publicKey.toBuffer(),
        wallet_ata.toBuffer(),
      ]);
      const message_hash = keccak256(buffer);
      const signature = keyPair.sign(message_hash, "hex");
      const { r, s, recoveryParam } = signature;
      const signatureBuffer = Buffer.concat([
        r.toArrayLike(Buffer, "be", 32),
        s.toArrayLike(Buffer, "be", 32),
      ]);
      await gatewayProgram.methods
        .withdrawSplToken(
          usdcDecimals,
          amount,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce2
        )
        .accounts({
          pdaAta: pda_ata,
          mintAccount: mint_fake.publicKey,
          recipientAta: wallet_ata,
          recipient: wallet.publicKey,
        })
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("ConstraintAssociated");
      const account4 = await spl.getAccount(conn, pda_ata);
      expect(account4.amount).to.be.eq(4_500_000n);
    }
  });

  it("Deposit if receiver is empty address should fail", async () => {
    try {
      await gatewayProgram.methods
        .deposit(new anchor.BN(1_000_000_000), Array(20).fill(0), revertOptions)
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("EmptyReceiver");
    }
  });

  it("Deposit through connected program", async () => {
    const balanceBefore = await conn.getBalance(pdaAccount);
    await connectedProgram.methods
      .triggerDeposit(
        new anchor.BN(1_000_000_000),
        Array.from(address),
        revertOptions
      )
      .accounts({
        gatewayPda: pdaAccount,
        gatewayProgram: gatewayProgram.programId,
      })
      .rpc();

    const balanceAfter = await conn.getBalance(pdaAccount);
    // amount + deposit fee
    expect(balanceAfter - balanceBefore).to.eq(1_000_000_000 + 2_000_000);
  });

  it("Deposit and withdraw 0.5 SOL from Gateway with ECDSA signature", async () => {
    const balanceBefore = await conn.getBalance(pdaAccount);
    await gatewayProgram.methods
      .deposit(new anchor.BN(1_000_000_000), Array.from(address), revertOptions)
      .rpc();
    let balanceAfter = await conn.getBalance(pdaAccount);
    // amount + deposit fee
    expect(balanceAfter - balanceBefore).to.eq(1_000_000_000 + 2_000_000);
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const amount = new anchor.BN(500000000);
    const to = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      wallet.publicKey
    );
    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x01]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      to.toBuffer(),
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    await gatewayProgram.methods
      .withdraw(
        amount,
        Array.from(signatureBuffer),
        Number(recoveryParam),
        Array.from(message_hash),
        nonce
      )
      .accounts({
        recipient: to,
      })
      .rpc();
    let bal2 = await conn.getBalance(pdaAccount);
    expect(bal2).to.be.eq(balanceAfter - 500_000_000);
    let bal3 = await conn.getBalance(to);
    expect(bal3).to.be.gte(500_000_000);
  });

  it("Withdraw with wrong nonce should fail", async () => {
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const amount = new anchor.BN(500000000);
    const to = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      wallet.publicKey
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x01]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      to.toBuffer(),
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      await gatewayProgram.methods
        .withdraw(
          amount,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce.subn(1)
        )
        .accounts({
          recipient: to,
        })
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("NonceMismatch.");
    }
  });

  it("Withdraw with wrong msg hash should fail", async () => {
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const amount = new anchor.BN(500000000);
    const to = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      wallet.publicKey
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x01]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.subn(1).toArrayLike(Buffer, "be", 8), // wrong nonce
      amount.toArrayLike(Buffer, "be", 8),
      to.toBuffer(),
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      await gatewayProgram.methods
        .withdraw(
          amount,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accounts({
          recipient: to,
        })
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("MessageHashMismatch");
    }
  });

  it("Withdraw with wrong signer should fail", async () => {
    const key = ec.genKeyPair(); // non TSS key pair
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const amount = new anchor.BN(500000000);
    const to = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      wallet.publicKey
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x01]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      to.toBuffer(),
    ]);
    const message_hash = keccak256(buffer);
    const signature = key.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      await gatewayProgram.methods
        .withdraw(
          amount,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accounts({
          recipient: to,
        })
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("TSSAuthenticationFailed");
    }
  });

  it("Calls execute and onCall", async () => {
    await connectedProgram.methods.initialize().rpc();
    await gatewayProgram.methods
      .deposit(new anchor.BN(1_000_000_000), Array.from(address), revertOptions)
      .rpc();

    const randomWallet = anchor.web3.Keypair.generate();
    const lastMessageData = "execute_sol";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedProgram.programId
    );
    const amount = new anchor.BN(500000000);

    // signature
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x05]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      connectedProgram.programId.toBuffer(),
      Buffer.from(Array.from(address)),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    // balances before call
    const connectedPdaBalanceBefore = await conn.getBalance(
      connectedPdaAccount
    );
    const randomWalletBalanceBefore = await conn.getBalance(
      randomWallet.publicKey
    );

    // call the `execute` function in the gateway program
    await gatewayProgram.methods
      .execute(
        amount,
        Array.from(address),
        data,
        Array.from(signatureBuffer),
        Number(recoveryParam),
        Array.from(message_hash),
        nonce
      )
      .accountsPartial({
        // mandatory predefined accounts
        signer: wallet.publicKey,
        pda: pdaAccount,
        destinationProgram: connectedProgram.programId,
        destinationProgramPda: connectedPdaAccount,
      })
      .remainingAccounts([
        // accounts coming from withdraw and call msg
        { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
        { pubkey: pdaAccount, isSigner: false, isWritable: false },
        { pubkey: randomWallet.publicKey, isSigner: false, isWritable: true },
        {
          pubkey: anchor.web3.SystemProgram.programId,
          isSigner: false,
          isWritable: false,
        },
        {
          pubkey: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
          isSigner: false,
          isWritable: false,
        },
      ])
      .rpc();

    const connectedPdaAfter = await connectedProgram.account.pda.fetch(
      connectedPdaAccount
    );

    // check connected pda state was updated
    expect(connectedPdaAfter.lastMessage).to.be.eq(lastMessageData);
    expect(Array.from(connectedPdaAfter.lastSender)).to.be.deep.eq(
      Array.from(address)
    );

    // check balances were updated
    const connectedPdaBalanceAfter = await conn.getBalance(connectedPdaAccount);
    const randomWalletBalanceAfter = await conn.getBalance(
      randomWallet.publicKey
    );

    expect(connectedPdaBalanceBefore + amount.toNumber() / 2).to.eq(
      connectedPdaBalanceAfter
    );
    expect(randomWalletBalanceBefore + amount.toNumber() / 2).to.eq(
      randomWalletBalanceAfter
    );
  });

  it("Calls execute and onCall reverts if connected program reverts", async () => {
    await gatewayProgram.methods
      .deposit(new anchor.BN(1_000_000_000), Array.from(address), revertOptions)
      .rpc();

    const randomWallet = anchor.web3.Keypair.generate();
    const lastMessageData = "revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedProgram.programId
    );
    const amount = new anchor.BN(500000000);

    // signature
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x05]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      connectedProgram.programId.toBuffer(),
      Buffer.from(Array.from(address)),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute` function in the gateway program
      await gatewayProgram.methods
        .execute(
          amount,
          Array.from(address),
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          destinationProgram: connectedProgram.programId,
          destinationProgramPda: connectedPdaAccount,
        })
        .remainingAccounts([
          // accounts coming from withdraw and call msg
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          { pubkey: randomWallet.publicKey, isSigner: false, isWritable: true },
          {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
    }
  });

  it("Calls execute and onCall reverts if wrong msg hash", async () => {
    await gatewayProgram.methods
      .deposit(new anchor.BN(1_000_000_000), Array.from(address), revertOptions)
      .rpc();

    const randomWallet = anchor.web3.Keypair.generate();
    const lastMessageData = "revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedProgram.programId
    );
    const amount = new anchor.BN(500000000);

    // signature
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x05]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.subn(1).toArrayLike(Buffer, "be", 8), // wrong nonce
      amount.toArrayLike(Buffer, "be", 8),
      connectedProgram.programId.toBuffer(),
      Buffer.from(Array.from(address)),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute` function in the gateway program
      await gatewayProgram.methods
        .execute(
          amount,
          Array.from(address),
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          destinationProgram: connectedProgram.programId,
          destinationProgramPda: connectedPdaAccount,
        })
        .remainingAccounts([
          // accounts coming from withdraw and call msg
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          { pubkey: randomWallet.publicKey, isSigner: false, isWritable: true },
          {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("MessageHashMismatch");
    }
  });

  it("Calls execute and onCall reverts if wrong signer", async () => {
    const key = ec.genKeyPair(); // non TSS key pair
    await gatewayProgram.methods
      .deposit(new anchor.BN(1_000_000_000), Array.from(address), revertOptions)
      .rpc();

    const randomWallet = anchor.web3.Keypair.generate();
    const lastMessageData = "revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedProgram.programId
    );
    const amount = new anchor.BN(500000000);

    // signature
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x05]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      connectedProgram.programId.toBuffer(),
      Buffer.from(Array.from(address)),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = key.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute` function in the gateway program
      await gatewayProgram.methods
        .execute(
          amount,
          Array.from(address),
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          destinationProgram: connectedProgram.programId,
          destinationProgramPda: connectedPdaAccount,
        })
        .remainingAccounts([
          // accounts coming from withdraw and call msg
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          { pubkey: randomWallet.publicKey, isSigner: false, isWritable: true },
          {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("TSSAuthenticationFailed");
    }
  });

  it("Calls execute and onCall reverts if signer is passed in remaining accounts", async () => {
    await gatewayProgram.methods
      .deposit(new anchor.BN(1_000_000_000), Array.from(address), revertOptions)
      .rpc();

    const randomWallet = anchor.web3.Keypair.generate();
    const lastMessageData = "revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedProgram.programId
    );
    const amount = new anchor.BN(500000000);

    // signature
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x05]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      connectedProgram.programId.toBuffer(),
      Buffer.from(Array.from(address)),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute` function in the gateway program
      await gatewayProgram.methods
        .execute(
          amount,
          Array.from(address),
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          destinationProgram: connectedProgram.programId,
          destinationProgramPda: connectedPdaAccount,
        })
        .remainingAccounts([
          // accounts coming from withdraw and call msg
          { pubkey: wallet.publicKey, isSigner: true, isWritable: true },
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          { pubkey: randomWallet.publicKey, isSigner: false, isWritable: true },
          {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("InvalidInstructionData");
    }
  });

  it("Calls execute and onRevert", async () => {
    const lastMessageData = "execute_rev_sol";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedProgram.programId
    );
    const amount = new anchor.BN(500000000);

    // signature
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x08]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      connectedProgram.programId.toBuffer(),
      random_account.publicKey.toBuffer(),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    // balances before call
    const connectedPdaBalanceBefore = await conn.getBalance(
      connectedPdaAccount
    );

    // call the `execute` function in the gateway program
    await gatewayProgram.methods
      .executeRevert(
        amount,
        random_account.publicKey,
        data,
        Array.from(signatureBuffer),
        Number(recoveryParam),
        Array.from(message_hash),
        nonce
      )
      .accountsPartial({
        // mandatory predefined accounts
        signer: wallet.publicKey,
        pda: pdaAccount,
        destinationProgram: connectedProgram.programId,
        destinationProgramPda: connectedPdaAccount,
      })
      .remainingAccounts([
        // accounts coming from withdraw and call msg
        { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
        { pubkey: pdaAccount, isSigner: false, isWritable: false },
        {
          pubkey: anchor.web3.SystemProgram.programId,
          isSigner: false,
          isWritable: false,
        },
      ])
      .rpc();

    const connectedPdaAfter = await connectedProgram.account.pda.fetch(
      connectedPdaAccount
    );

    // check connected pda state was updated
    expect(connectedPdaAfter.lastRevertMessage).to.be.eq(lastMessageData);
    expect(connectedPdaAfter.lastRevertSender.toString()).to.be.eq(
      random_account.publicKey.toString()
    );

    // check balances were updated
    const connectedPdaBalanceAfter = await conn.getBalance(connectedPdaAccount);
    expect(connectedPdaBalanceBefore + amount.toNumber()).to.eq(
      connectedPdaBalanceAfter
    );
  });

  it("Calls execute and onRevert reverts if connected program reverts", async () => {
    await gatewayProgram.methods
      .deposit(new anchor.BN(1_000_000_000), Array.from(address), revertOptions)
      .rpc();

    const lastMessageData = "revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedProgram.programId
    );
    const amount = new anchor.BN(500000000);

    // signature
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x08]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      connectedProgram.programId.toBuffer(),
      random_account.publicKey.toBuffer(),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute` function in the gateway program
      await gatewayProgram.methods
        .executeRevert(
          amount,
          random_account.publicKey,
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          destinationProgram: connectedProgram.programId,
          destinationProgramPda: connectedPdaAccount,
        })
        .remainingAccounts([
          // accounts coming from withdraw and call msg
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
    }
  });

  it("Calls execute and onRevert reverts if wrong msg hash", async () => {
    await gatewayProgram.methods
      .deposit(new anchor.BN(1_000_000_000), Array.from(address), revertOptions)
      .rpc();

    const lastMessageData = "revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedProgram.programId
    );
    const amount = new anchor.BN(500000000);

    // signature
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x08]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.subn(1).toArrayLike(Buffer, "be", 8), // wrong nonce
      amount.toArrayLike(Buffer, "be", 8),
      connectedProgram.programId.toBuffer(),
      random_account.publicKey.toBuffer(),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute` function in the gateway program
      await gatewayProgram.methods
        .executeRevert(
          amount,
          random_account.publicKey,
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          destinationProgram: connectedProgram.programId,
          destinationProgramPda: connectedPdaAccount,
        })
        .remainingAccounts([
          // accounts coming from withdraw and call msg
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("MessageHashMismatch");
    }
  });

  it("Calls execute and onRevert reverts if wrong signer", async () => {
    const key = ec.genKeyPair(); // non TSS key pair
    await gatewayProgram.methods
      .deposit(new anchor.BN(1_000_000_000), Array.from(address), revertOptions)
      .rpc();

    const lastMessageData = "revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedProgram.programId
    );
    const amount = new anchor.BN(500000000);

    // signature
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x08]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      connectedProgram.programId.toBuffer(),
      random_account.publicKey.toBuffer(),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = key.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute` function in the gateway program
      await gatewayProgram.methods
        .executeRevert(
          amount,
          random_account.publicKey,
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          destinationProgram: connectedProgram.programId,
          destinationProgramPda: connectedPdaAccount,
        })
        .remainingAccounts([
          // accounts coming from withdraw and call msg
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("TSSAuthenticationFailed");
    }
  });

  it("Calls execute and onRevert reverts if signer is passed in remaining accounts", async () => {
    await gatewayProgram.methods
      .deposit(new anchor.BN(1_000_000_000), Array.from(address), revertOptions)
      .rpc();

    const lastMessageData = "revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedProgram.programId
    );
    const amount = new anchor.BN(500000000);

    // signature
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;
    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x08]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      connectedProgram.programId.toBuffer(),
      random_account.publicKey.toBuffer(),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute` function in the gateway program
      await gatewayProgram.methods
        .executeRevert(
          amount,
          random_account.publicKey,
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          destinationProgram: connectedProgram.programId,
          destinationProgramPda: connectedPdaAccount,
        })
        .remainingAccounts([
          // accounts coming from withdraw and call msg
          { pubkey: wallet.publicKey, isSigner: true, isWritable: true },
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("InvalidInstructionData");
    }
  });

  it("Calls execute spl token and onCall", async () => {
    await connectedSPLProgram.methods.initialize().rpc();

    const randomWallet = anchor.web3.Keypair.generate();
    let randomWalletAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      randomWallet.publicKey,
      true
    );
    const lastMessageData = "execute_spl";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedSPLProgram.programId
    );
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;

    let destinationPdaAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      connectedPdaAccount,
      true
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x06]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
      destinationPdaAta.address.toBuffer(),
      Buffer.from(Array.from(address)),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    // call the `execute_spl_token` function in the gateway program
    await gatewayProgram.methods
      .executeSplToken(
        usdcDecimals,
        amount,
        Array.from(address),
        data,
        Array.from(signatureBuffer),
        Number(recoveryParam),
        Array.from(message_hash),
        nonce
      )
      .accountsPartial({
        // mandatory predefined accounts
        signer: wallet.publicKey,
        pda: pdaAccount,
        pdaAta: pda_ata,
        mintAccount: mint.publicKey,
        destinationProgram: connectedSPLProgram.programId,
        destinationProgramPda: connectedPdaAccount,
        destinationProgramPdaAta: destinationPdaAta.address,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
        associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
      })
      .remainingAccounts([
        // accounts coming from withdraw and call msg
        { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
        {
          pubkey: destinationPdaAta.address,
          isSigner: false,
          isWritable: true,
        },
        { pubkey: mint.publicKey, isSigner: false, isWritable: false },
        { pubkey: pdaAccount, isSigner: false, isWritable: false },
        { pubkey: randomWallet.publicKey, isSigner: false, isWritable: false },
        { pubkey: randomWalletAta.address, isSigner: false, isWritable: true },
        {
          pubkey: spl.TOKEN_PROGRAM_ID,
          isSigner: false,
          isWritable: false,
        },
        {
          pubkey: SYSTEM_PROGRAM_ID,
          isSigner: false,
          isWritable: false,
        },
      ])
      .rpc();

    const connectedPdaAfter = await connectedSPLProgram.account.pda.fetch(
      connectedPdaAccount
    );

    // check connected pda state was updated
    expect(connectedPdaAfter.lastMessage).to.be.eq(lastMessageData);
    expect(Array.from(connectedPdaAfter.lastSender)).to.be.deep.eq(
      Array.from(address)
    );

    // check amount was split between connected pda and random wallet ata
    const destinationPdaAtaAcc = await spl.getAccount(
      conn,
      destinationPdaAta.address
    );
    const randomWalletAtaAcc = await spl.getAccount(
      conn,
      randomWalletAta.address
    );

    expect(destinationPdaAtaAcc.amount).to.be.eq(250000n);
    expect(randomWalletAtaAcc.amount).to.be.eq(250000n);
  });

  it("Calls execute spl token and onCall reverts if connected program reverts", async () => {
    const randomWallet = anchor.web3.Keypair.generate();
    let randomWalletAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      randomWallet.publicKey,
      true
    );
    const lastMessageData = "revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedSPLProgram.programId
    );
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;

    let destinationPdaAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      connectedPdaAccount,
      true
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x06]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
      destinationPdaAta.address.toBuffer(),
      Buffer.from(Array.from(address)),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute_spl_token` function in the gateway program
      await gatewayProgram.methods
        .executeSplToken(
          usdcDecimals,
          amount,
          Array.from(address),
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          pdaAta: pda_ata,
          mintAccount: mint.publicKey,
          destinationProgram: connectedSPLProgram.programId,
          destinationProgramPda: connectedPdaAccount,
          destinationProgramPdaAta: destinationPdaAta.address,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SYSTEM_PROGRAM_ID,
        })
        .remainingAccounts([
          // accounts coming from withdraw and call msg
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          {
            pubkey: destinationPdaAta.address,
            isSigner: false,
            isWritable: true,
          },
          { pubkey: mint.publicKey, isSigner: false, isWritable: false },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          {
            pubkey: randomWallet.publicKey,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: randomWalletAta.address,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: spl.TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: SYSTEM_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
    }
  });

  it("Calls execute spl token and onCall reverts if signer is passed in remaining accounts", async () => {
    const randomWallet = anchor.web3.Keypair.generate();
    let randomWalletAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      randomWallet.publicKey,
      true
    );
    const lastMessageData = "revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedSPLProgram.programId
    );
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;

    let destinationPdaAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      connectedPdaAccount,
      true
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x06]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
      destinationPdaAta.address.toBuffer(),
      Buffer.from(Array.from(address)),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute_spl_token` function in the gateway program
      await gatewayProgram.methods
        .executeSplToken(
          usdcDecimals,
          amount,
          Array.from(address),
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          pdaAta: pda_ata,
          mintAccount: mint.publicKey,
          destinationProgram: connectedSPLProgram.programId,
          destinationProgramPda: connectedPdaAccount,
          destinationProgramPdaAta: destinationPdaAta.address,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SYSTEM_PROGRAM_ID,
        })
        .remainingAccounts([
          // accounts coming from withdraw and call msg
          { pubkey: wallet.publicKey, isSigner: true, isWritable: true },
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          {
            pubkey: destinationPdaAta.address,
            isSigner: false,
            isWritable: true,
          },
          { pubkey: mint.publicKey, isSigner: false, isWritable: false },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          {
            pubkey: randomWallet.publicKey,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: randomWalletAta.address,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: spl.TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: SYSTEM_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("InvalidInstructionData");
    }
  });

  it("Calls execute spl token and onCall reverts if wrong msg hash", async () => {
    const randomWallet = anchor.web3.Keypair.generate();
    let randomWalletAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      randomWallet.publicKey,
      true
    );
    const lastMessageData = "revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedSPLProgram.programId
    );
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;

    let destinationPdaAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      connectedPdaAccount,
      true
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x06]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.subn(1).toArrayLike(Buffer, "be", 8), // wrong nonce
      amount.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
      destinationPdaAta.address.toBuffer(),
      Buffer.from(Array.from(address)),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute_spl_token` function in the gateway program
      await gatewayProgram.methods
        .executeSplToken(
          usdcDecimals,
          amount,
          Array.from(address),
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          pdaAta: pda_ata,
          mintAccount: mint.publicKey,
          destinationProgram: connectedSPLProgram.programId,
          destinationProgramPda: connectedPdaAccount,
          destinationProgramPdaAta: destinationPdaAta.address,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SYSTEM_PROGRAM_ID,
        })
        .remainingAccounts([
          // accounts coming from withdraw and call msg
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          {
            pubkey: destinationPdaAta.address,
            isSigner: false,
            isWritable: true,
          },
          { pubkey: mint.publicKey, isSigner: false, isWritable: false },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          {
            pubkey: randomWallet.publicKey,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: randomWalletAta.address,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: spl.TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: SYSTEM_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("MessageHashMismatch");
    }
  });

  it("Calls execute spl token and onCall reverts if wrong signer", async () => {
    const key = ec.genKeyPair(); // non TSS key pair
    const randomWallet = anchor.web3.Keypair.generate();
    let randomWalletAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      randomWallet.publicKey,
      true
    );
    const lastMessageData = "revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedSPLProgram.programId
    );
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;

    let destinationPdaAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      connectedPdaAccount,
      true
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x06]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
      destinationPdaAta.address.toBuffer(),
      Buffer.from(Array.from(address)),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = key.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute_spl_token` function in the gateway program
      await gatewayProgram.methods
        .executeSplToken(
          usdcDecimals,
          amount,
          Array.from(address),
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          pdaAta: pda_ata,
          mintAccount: mint.publicKey,
          destinationProgram: connectedSPLProgram.programId,
          destinationProgramPda: connectedPdaAccount,
          destinationProgramPdaAta: destinationPdaAta.address,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SYSTEM_PROGRAM_ID,
        })
        .remainingAccounts([
          // accounts coming from withdraw and call msg
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          {
            pubkey: destinationPdaAta.address,
            isSigner: false,
            isWritable: true,
          },
          { pubkey: mint.publicKey, isSigner: false, isWritable: false },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          {
            pubkey: randomWallet.publicKey,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: randomWalletAta.address,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: spl.TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: SYSTEM_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("TSSAuthenticationFailed");
    }
  });

  it("Calls execute spl token and onRevert", async () => {
    const randomWallet = anchor.web3.Keypair.generate();
    const lastMessageData = "execute_spl";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedSPLProgram.programId
    );
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;

    let destinationPdaAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      connectedPdaAccount,
      true
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x09]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
      destinationPdaAta.address.toBuffer(),
      randomWallet.publicKey.toBuffer(),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    // balances before
    const destinationPdaAtaAccBefore = await spl.getAccount(
      conn,
      destinationPdaAta.address
    );

    // call the `execute_spl_token_revert` function in the gateway program
    await gatewayProgram.methods
      .executeSplTokenRevert(
        usdcDecimals,
        amount,
        randomWallet.publicKey,
        data,
        Array.from(signatureBuffer),
        Number(recoveryParam),
        Array.from(message_hash),
        nonce
      )
      .accountsPartial({
        // mandatory predefined accounts
        signer: wallet.publicKey,
        pda: pdaAccount,
        pdaAta: pda_ata,
        mintAccount: mint.publicKey,
        destinationProgram: connectedSPLProgram.programId,
        destinationProgramPda: connectedPdaAccount,
        destinationProgramPdaAta: destinationPdaAta.address,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
        associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
      })
      .remainingAccounts([
        // accounts coming from revert msg
        { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
        {
          pubkey: destinationPdaAta.address,
          isSigner: false,
          isWritable: true,
        },
        { pubkey: mint.publicKey, isSigner: false, isWritable: false },
        { pubkey: pdaAccount, isSigner: false, isWritable: false },
        {
          pubkey: spl.TOKEN_PROGRAM_ID,
          isSigner: false,
          isWritable: false,
        },
        {
          pubkey: SYSTEM_PROGRAM_ID,
          isSigner: false,
          isWritable: false,
        },
      ])
      .rpc();

    const connectedPdaAfter = await connectedSPLProgram.account.pda.fetch(
      connectedPdaAccount
    );

    // check connected pda state was updated
    expect(connectedPdaAfter.lastRevertMessage).to.be.eq(lastMessageData);
    expect(connectedPdaAfter.lastRevertSender.toString()).to.be.deep.eq(
      randomWallet.publicKey.toString()
    );

    // check balances were updated
    const destinationPdaAtaAcc = await spl.getAccount(
      conn,
      destinationPdaAta.address
    );

    expect(Number(destinationPdaAtaAcc.amount.toString())).to.be.eq(
      Number(destinationPdaAtaAccBefore.amount) + amount.toNumber()
    );
  });

  it("Calls execute spl token and onRevert reverts if connected program reverts", async () => {
    const randomWallet = anchor.web3.Keypair.generate();
    const lastMessageData = "execute_spl_revert";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedSPLProgram.programId
    );
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;

    let destinationPdaAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      connectedPdaAccount,
      true
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x09]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
      destinationPdaAta.address.toBuffer(),
      randomWallet.publicKey.toBuffer(),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute_spl_token_revert` function in the gateway program
      await gatewayProgram.methods
        .executeSplTokenRevert(
          usdcDecimals,
          amount,
          randomWallet.publicKey,
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          pdaAta: pda_ata,
          mintAccount: mint.publicKey,
          destinationProgram: connectedSPLProgram.programId,
          destinationProgramPda: connectedPdaAccount,
          destinationProgramPdaAta: destinationPdaAta.address,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SYSTEM_PROGRAM_ID,
        })
        .remainingAccounts([
          // accounts coming from revert msg
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          {
            pubkey: destinationPdaAta.address,
            isSigner: false,
            isWritable: true,
          },
          { pubkey: mint.publicKey, isSigner: false, isWritable: false },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          {
            pubkey: spl.TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: SYSTEM_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
    }
  });

  it("Calls execute spl token and onRevert reverts if signer is passed in remaining accounts", async () => {
    const randomWallet = anchor.web3.Keypair.generate();
    const lastMessageData = "execute_spl";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedSPLProgram.programId
    );
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;

    let destinationPdaAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      connectedPdaAccount,
      true
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x09]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
      destinationPdaAta.address.toBuffer(),
      randomWallet.publicKey.toBuffer(),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute_spl_token_revert` function in the gateway program
      await gatewayProgram.methods
        .executeSplTokenRevert(
          usdcDecimals,
          amount,
          randomWallet.publicKey,
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          pdaAta: pda_ata,
          mintAccount: mint.publicKey,
          destinationProgram: connectedSPLProgram.programId,
          destinationProgramPda: connectedPdaAccount,
          destinationProgramPdaAta: destinationPdaAta.address,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SYSTEM_PROGRAM_ID,
        })
        .remainingAccounts([
          // accounts coming from revert msg
          { pubkey: wallet.publicKey, isSigner: true, isWritable: true },
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          {
            pubkey: destinationPdaAta.address,
            isSigner: false,
            isWritable: true,
          },
          { pubkey: mint.publicKey, isSigner: false, isWritable: false },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          {
            pubkey: spl.TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: SYSTEM_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err.message).to.include("InvalidInstructionData");
    }
  });

  it("Calls execute spl token and onRevert reverts if wrong message hash", async () => {
    const randomWallet = anchor.web3.Keypair.generate();
    const lastMessageData = "execute_spl";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedSPLProgram.programId
    );
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;

    let destinationPdaAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      connectedPdaAccount,
      true
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x09]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.subn(1).toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
      destinationPdaAta.address.toBuffer(),
      randomWallet.publicKey.toBuffer(),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute_spl_token_revert` function in the gateway program
      await gatewayProgram.methods
        .executeSplTokenRevert(
          usdcDecimals,
          amount,
          randomWallet.publicKey,
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          pdaAta: pda_ata,
          mintAccount: mint.publicKey,
          destinationProgram: connectedSPLProgram.programId,
          destinationProgramPda: connectedPdaAccount,
          destinationProgramPdaAta: destinationPdaAta.address,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SYSTEM_PROGRAM_ID,
        })
        .remainingAccounts([
          // accounts coming from revert msg
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          {
            pubkey: destinationPdaAta.address,
            isSigner: false,
            isWritable: true,
          },
          { pubkey: mint.publicKey, isSigner: false, isWritable: false },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          {
            pubkey: spl.TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: SYSTEM_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err.message).to.include("MessageHashMismatch");
    }
  });

  it("Calls execute spl token and onRevert reverts if wrong signer", async () => {
    const key = ec.genKeyPair(); // non TSS key pair
    const randomWallet = anchor.web3.Keypair.generate();
    const lastMessageData = "execute_spl";
    const data = Buffer.from(lastMessageData, "utf-8");
    let seeds = [Buffer.from("connected", "utf-8")];
    const [connectedPdaAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      connectedSPLProgram.programId
    );
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;

    let destinationPdaAta = await spl.getOrCreateAssociatedTokenAccount(
      conn,
      wallet,
      mint.publicKey,
      connectedPdaAccount,
      true
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x09]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
      destinationPdaAta.address.toBuffer(),
      randomWallet.publicKey.toBuffer(),
      data,
    ]);
    const message_hash = keccak256(buffer);
    const signature = key.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      // call the `execute_spl_token_revert` function in the gateway program
      await gatewayProgram.methods
        .executeSplTokenRevert(
          usdcDecimals,
          amount,
          randomWallet.publicKey,
          data,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accountsPartial({
          // mandatory predefined accounts
          signer: wallet.publicKey,
          pda: pdaAccount,
          pdaAta: pda_ata,
          mintAccount: mint.publicKey,
          destinationProgram: connectedSPLProgram.programId,
          destinationProgramPda: connectedPdaAccount,
          destinationProgramPdaAta: destinationPdaAta.address,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SYSTEM_PROGRAM_ID,
        })
        .remainingAccounts([
          // accounts coming from revert msg
          { pubkey: connectedPdaAccount, isSigner: false, isWritable: true },
          {
            pubkey: destinationPdaAta.address,
            isSigner: false,
            isWritable: true,
          },
          { pubkey: mint.publicKey, isSigner: false, isWritable: false },
          { pubkey: pdaAccount, isSigner: false, isWritable: false },
          {
            pubkey: spl.TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: SYSTEM_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err.message).to.include("TSSAuthenticationFailed");
    }
  });

  it("Withdraw SPL token to a non-existent account should succeed by creating it", async () => {
    let rentPayerPdaBal0 = await conn.getBalance(pdaAccount);
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;
    const wallet2 = anchor.web3.Keypair.generate();
    const to = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      wallet2.publicKey
    );

    let to_ata_bal = await conn.getBalance(to);
    expect(to_ata_bal).to.be.eq(0); // the new ata account (owned by wallet2) should be non-existent;
    await withdrawSplToken(
      mint,
      usdcDecimals,
      amount,
      nonce,
      pda_ata,
      to,
      wallet2.publicKey,
      gatewayProgram
    );
    to_ata_bal = await conn.getBalance(to);
    expect(to_ata_bal).to.be.gt(2_000_000); // the new ata account (owned by wallet2) should be created

    // pda should have reduced balance
    let rentPayerPdaBal1 = await conn.getBalance(pdaAccount);
    // expected reimbursement to be gas fee (5000 lamports) + ATA creation cost 2039280 lamports
    expect(rentPayerPdaBal0 - rentPayerPdaBal1).to.be.eq(to_ata_bal + 5000); // rentPayer pays rent
  });

  it("Withdraw SPL token with wrong nonce should fail", async () => {
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;
    const wallet2 = anchor.web3.Keypair.generate();
    const to = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      wallet2.publicKey
    );

    try {
      await withdrawSplToken(
        mint,
        usdcDecimals,
        amount,
        nonce.subn(1),
        pda_ata,
        to,
        wallet2.publicKey,
        gatewayProgram
      );
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("NonceMismatch.");
    }
  });

  it("Withdraw SPL token with wrong msg hash should fail", async () => {
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;
    const wallet2 = anchor.web3.Keypair.generate();
    const to = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      wallet2.publicKey
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x02]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.subn(1).toArrayLike(Buffer, "be", 8), // wrong nonce
      amount.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
      to.toBuffer(),
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);
    try {
      await gatewayProgram.methods
        .withdrawSplToken(
          usdcDecimals,
          amount,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accounts({
          pdaAta: pda_ata,
          mintAccount: mint.publicKey,
          recipientAta: to,
          recipient: wallet2.publicKey,
        })
        .rpc({ commitment: "processed" });
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("MessageHashMismatch");
    }
  });

  it("Withdraw SPL token with wrong signer should fail", async () => {
    const key = ec.genKeyPair(); // non TSS key pair
    let pda_ata = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      pdaAccount,
      true
    );
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const amount = new anchor.BN(500_000);
    const nonce = pdaAccountData.nonce;
    const wallet2 = anchor.web3.Keypair.generate();
    const to = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      wallet2.publicKey
    );

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x02]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      amount.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
      to.toBuffer(),
    ]);
    const message_hash = keccak256(buffer);
    const signature = key.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);
    try {
      await gatewayProgram.methods
        .withdrawSplToken(
          usdcDecimals,
          amount,
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accounts({
          pdaAta: pda_ata,
          mintAccount: mint.publicKey,
          recipientAta: to,
          recipient: wallet2.publicKey,
        })
        .rpc({ commitment: "processed" });
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("TSSAuthenticationFailed");
    }
  });

  it("Deposit and call with empty address receiver should fail", async () => {
    try {
      await gatewayProgram.methods
        .depositAndCall(
          new anchor.BN(1_000_000_000),
          Array(20).fill(0),
          Buffer.from("hello", "utf-8"),
          revertOptions
        )
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("EmptyReceiver");
    }
  });

  it("Deposit and call with above max payload size should fail", async () => {
    try {
      await gatewayProgram.methods
        .depositAndCall(
          new anchor.BN(1_000_000_000),
          Array.from(address),
          Buffer.from(Array(maxPayloadSize + 1).fill(1)),
          revertOptions
        )
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("MemoLengthExceeded");
    }
  });

  it("Call with empty address receiver should fail", async () => {
    try {
      await gatewayProgram.methods
        .call(Array(20).fill(0), Buffer.from("hello", "utf-8"), revertOptions)
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("EmptyReceiver");
    }
  });

  it("Call with above max payload size should fail", async () => {
    try {
      await gatewayProgram.methods
        .call(
          Array.from(address),
          Buffer.from(Array(maxPayloadSize + 1).fill(1)),
          revertOptions
        )
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("MemoLengthExceeded");
    }
  });

  it("Call with max payload size", async () => {
    const txsig = await gatewayProgram.methods
      .call(
        Array.from(address),
        Buffer.from(Array(maxPayloadSize).fill(1)),
        revertOptions
      )
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 400000 }),
      ])
      .rpc({ commitment: "processed" });
    await conn.getParsedTransaction(txsig, "confirmed");
  });

  it("Deposit and call with max payload size", async () => {
    const bal1 = await conn.getBalance(pdaAccount);
    const txsig = await gatewayProgram.methods
      .depositAndCall(
        new anchor.BN(1_000_000_000),
        Array.from(address),
        Buffer.from(Array(maxPayloadSize).fill(1)),
        revertOptions
      )
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 400000 }),
      ])
      .rpc({ commitment: "processed" });
    await conn.getParsedTransaction(txsig, "confirmed");
    const bal2 = await conn.getBalance(pdaAccount);
    expect(bal2 - bal1).to.be.gte(1_000_000_000);
  });

  it("Deposit and call", async () => {
    let bal1 = await conn.getBalance(pdaAccount);
    const txsig = await gatewayProgram.methods
      .depositAndCall(
        new anchor.BN(1_000_000_000),
        Array.from(address),
        Buffer.from("hello", "utf-8"),
        revertOptions
      )
      .rpc({ commitment: "processed" });
    await conn.getParsedTransaction(txsig, "confirmed");
    let bal2 = await conn.getBalance(pdaAccount);
    expect(bal2 - bal1).to.be.gte(1_000_000_000);
  });

  it("Deposit SPL with empty address receiver should fail", async () => {
    try {
      await depositSplTokens(
        gatewayProgram,
        conn,
        wallet,
        mint,
        Buffer.alloc(20)
      );
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("EmptyReceiver");
    }
  });

  it("Unwhitelist SPL token and deposit should fail", async () => {
    await gatewayProgram.methods
      .unwhitelistSplMint([], 0, [], new anchor.BN(0))
      .accounts({
        whitelistCandidate: mint.publicKey,
      })
      .rpc();

    try {
      await depositSplTokens(gatewayProgram, conn, wallet, mint, address);
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("AccountNotInitialized");
    }
  });

  it("Re-whitelist SPL token and deposit should succeed", async () => {
    await gatewayProgram.methods
      .whitelistSplMint([], 0, [], new anchor.BN(0))
      .accounts({
        whitelistCandidate: mint.publicKey,
      })
      .rpc();
    await depositSplTokens(gatewayProgram, conn, wallet, mint, address);
  });

  it("Unwhitelist SPL token using TSS signature and deposit should fail", async () => {
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x04]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    await gatewayProgram.methods
      .unwhitelistSplMint(
        Array.from(signatureBuffer),
        Number(recoveryParam),
        Array.from(message_hash),
        nonce
      )
      .accounts({
        whitelistCandidate: mint.publicKey,
      })
      .rpc();

    try {
      await depositSplTokens(gatewayProgram, conn, wallet, mint, address);
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("AccountNotInitialized");
    }
  });

  it("Re-whitelist SPL token using TSS signature and deposit should succeed", async () => {
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x03]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    await gatewayProgram.methods
      .whitelistSplMint(
        Array.from(signatureBuffer),
        Number(recoveryParam),
        Array.from(message_hash),
        nonce
      )
      .accounts({
        whitelistCandidate: mint.publicKey,
      })
      .rpc();
    await depositSplTokens(gatewayProgram, conn, wallet, mint, address);
  });

  it("Unwhitelist SPL token using wrong msg hash should fail", async () => {
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x03]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.subn(1).toArrayLike(Buffer, "be", 8), // wrong nonce
      mint.publicKey.toBuffer(),
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      await gatewayProgram.methods
        .unwhitelistSplMint(
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accounts({
          whitelistCandidate: mint.publicKey,
        })
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("MessageHashMismatch");
    }
  });

  it("Unwhitelist SPL token using wrong signer should fail", async () => {
    const key = ec.genKeyPair(); // non TSS key pair
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x04]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
    ]);
    const message_hash = keccak256(buffer);
    const signature = key.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      await gatewayProgram.methods
        .unwhitelistSplMint(
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce
        )
        .accounts({
          whitelistCandidate: mint.publicKey,
        })
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("TSSAuthenticationFailed");
    }
  });

  it("Unwhitelist SPL token using wrong nonce should fail", async () => {
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    const nonce = pdaAccountData.nonce;

    const buffer = Buffer.concat([
      Buffer.from("ZETACHAIN", "utf-8"),
      Buffer.from([0x03]),
      chain_id_bn.toArrayLike(Buffer, "be", 8),
      nonce.toArrayLike(Buffer, "be", 8),
      mint.publicKey.toBuffer(),
    ]);
    const message_hash = keccak256(buffer);
    const signature = keyPair.sign(message_hash, "hex");
    const { r, s, recoveryParam } = signature;
    const signatureBuffer = Buffer.concat([
      r.toArrayLike(Buffer, "be", 32),
      s.toArrayLike(Buffer, "be", 32),
    ]);

    try {
      await gatewayProgram.methods
        .unwhitelistSplMint(
          Array.from(signatureBuffer),
          Number(recoveryParam),
          Array.from(message_hash),
          nonce.subn(1)
        )
        .accounts({
          whitelistCandidate: mint.publicKey,
        })
        .rpc();
      throw new Error("Expected error not thrown"); // This line will make the test fail if no error is thrown
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("NonceMismatch.");
    }
  });

  it("Update TSS address", async () => {
    const newTss = new Uint8Array(20);
    randomFillSync(newTss);
    await gatewayProgram.methods.updateTss(Array.from(newTss)).rpc();
    const pdaAccountData = await gatewayProgram.account.pda.fetch(pdaAccount);
    expect(pdaAccountData.tssAddress).to.be.deep.eq(Array.from(newTss));
    expect(pdaAccountData.nonce.toNumber()).to.eq(0);

    // only the authority stored in PDA can update the TSS address; the following should fail
    try {
      await gatewayProgram.methods
        .updateTss(Array.from(newTss))
        .accounts({
          signer: mint.publicKey,
        })
        .signers([mint])
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("SignerIsNotAuthority");
    }
  });

  it("Pause deposit and deposit should fail", async () => {
    const newTss = new Uint8Array(20);
    randomFillSync(newTss);
    await gatewayProgram.methods.setDepositPaused(true).rpc();

    // now try deposit, should fail
    try {
      await gatewayProgram.methods
        .depositAndCall(
          new anchor.BN(1_000_000),
          Array.from(address),
          Buffer.from("hi", "utf-8"),
          revertOptions
        )
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("DepositPaused");
    }
  });

  it("Reset nonce", async () => {
    await gatewayProgram.methods.resetNonce(new anchor.BN(1000)).rpc();
    const pdaAccountDataAfter = await gatewayProgram.account.pda.fetch(
      pdaAccount
    );
    expect(pdaAccountDataAfter.nonce.toNumber()).to.equal(1000);
  });

  const newAuthority = anchor.web3.Keypair.generate();
  it("Update authority", async () => {
    await gatewayProgram.methods.updateAuthority(newAuthority.publicKey).rpc();
    // now the old authority cannot update TSS address and will fail
    try {
      await gatewayProgram.methods
        .updateTss(Array.from(new Uint8Array(20)))
        .rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("SignerIsNotAuthority");
    }
  });

  it("Reset nonce fails if wrong authority", async () => {
    try {
      await gatewayProgram.methods.resetNonce(new anchor.BN(1000)).rpc();
      throw new Error("Expected error not thrown");
    } catch (err) {
      expect(err).to.be.instanceof(anchor.AnchorError);
      expect(err.message).to.include("SignerIsNotAuthority");
    }
  });
});
