import { Program, BN } from "@coral-xyz/anchor";
import { ConnectedSpl } from "../../target/types/connected_spl";
import {
  Connection,
  ConfirmOptions,
  PublicKey,
  Keypair,
  Signer,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  getAuthAddress,
  getPoolAddress,
  getPoolLpMintAddress,
  getPoolVaultAddress,
  createTokenMintAndAssociatedTokenAccount,
  getOrcleAccountAddress,
} from "./index";

import { cpSwapProgram, configAddress, createPoolFeeReceive } from "../config";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { CpmmPoolInfoLayout } from "@raydium-io/raydium-sdk-v2";

export async function setupSwapTest(
  program: Program<ConnectedSpl>,
  connection: Connection,
  owner: Signer,
  transferFeeConfig: { transferFeeBasisPoints: number; MaxFee: number } = {
    transferFeeBasisPoints: 0,
    MaxFee: 0,
  },
  confirmOptions?: ConfirmOptions
) {
  const [{ token0, token0Program }, { token1, token1Program }] =
    await createTokenMintAndAssociatedTokenAccount(
      connection,
      owner,
      new Keypair(),
      transferFeeConfig
    );

  const { cpSwapPoolState } = await initialize(
    program,
    owner,
    configAddress,
    token0,
    token0Program,
    token1,
    token1Program,
    confirmOptions
  );

  await deposit(
    program,
    owner,
    configAddress,
    token0,
    token0Program,
    token1,
    token1Program,
    new BN(100_000),
    new BN(1_000_000),
    new BN(1_000_000),
    confirmOptions
  );
  return cpSwapPoolState;
}

export async function initialize(
  program: Program<ConnectedSpl>,
  creator: Signer,
  configAddress: PublicKey,
  token0: PublicKey,
  token0Program: PublicKey,
  token1: PublicKey,
  token1Program: PublicKey,
  confirmOptions?: ConfirmOptions,
  initAmount: { initAmount0: BN; initAmount1: BN } = {
    initAmount0: new BN(100_000),
    initAmount1: new BN(200_000),
  },
  createPoolFee = createPoolFeeReceive
) {
  const [auth] = await getAuthAddress(cpSwapProgram);
  const [poolAddress] = await getPoolAddress(
    configAddress,
    token0,
    token1,
    cpSwapProgram
  );
  const [lpMintAddress] = await getPoolLpMintAddress(
    poolAddress,
    cpSwapProgram
  );
  const [vault0] = await getPoolVaultAddress(
    poolAddress,
    token0,
    cpSwapProgram
  );
  const [vault1] = await getPoolVaultAddress(
    poolAddress,
    token1,
    cpSwapProgram
  );
  const [creatorLpTokenAddress] = await PublicKey.findProgramAddress(
    [
      creator.publicKey.toBuffer(),
      TOKEN_PROGRAM_ID.toBuffer(),
      lpMintAddress.toBuffer(),
    ],
    ASSOCIATED_PROGRAM_ID
  );

  const [observationAddress] = await getOrcleAccountAddress(
    poolAddress,
    cpSwapProgram
  );

  const creatorToken0 = getAssociatedTokenAddressSync(
    token0,
    creator.publicKey,
    false,
    token0Program
  );
  const creatorToken1 = getAssociatedTokenAddressSync(
    token1,
    creator.publicKey,
    false,
    token1Program
  );
  const tx = await program.methods
    .proxyInitialize(initAmount.initAmount0, initAmount.initAmount1, new BN(0))
    .accountsPartial({
      cpSwapProgram: cpSwapProgram,
      creator: creator.publicKey,
      ammConfig: configAddress,
      authority: auth,
      poolState: poolAddress,
      token0Mint: token0,
      token1Mint: token1,
      lpMint: lpMintAddress,
      creatorToken0,
      creatorToken1,
      creatorLpToken: creatorLpTokenAddress,
      token0Vault: vault0,
      token1Vault: vault1,
      createPoolFee,
      observationState: observationAddress,
      tokenProgram: TOKEN_PROGRAM_ID,
      token0Program: token0Program,
      token1Program: token1Program,
      systemProgram: SystemProgram.programId,
      rent: SYSVAR_RENT_PUBKEY,
    })
    .preInstructions([
      ComputeBudgetProgram.setComputeUnitLimit({ units: 400000 }),
    ])
    .rpc(confirmOptions);
  const accountInfo = await program.provider.connection.getAccountInfo(
    poolAddress
  );
  const poolState = CpmmPoolInfoLayout.decode(accountInfo.data);
  const cpSwapPoolState = {
    ammConfig: poolState.configId,
    token0Mint: poolState.mintA,
    token0Program: poolState.mintProgramA,
    token1Mint: poolState.mintB,
    token1Program: poolState.mintProgramB,
  };
  return { poolAddress, cpSwapPoolState, tx };
}

export async function deposit(
  program: Program<ConnectedSpl>,
  owner: Signer,
  configAddress: PublicKey,
  token0: PublicKey,
  token0Program: PublicKey,
  token1: PublicKey,
  token1Program: PublicKey,
  lp_token_amount: BN,
  maximum_token_0_amount: BN,
  maximum_token_1_amount: BN,
  confirmOptions?: ConfirmOptions
) {
  const [auth] = await getAuthAddress(cpSwapProgram);
  const [poolAddress] = await getPoolAddress(
    configAddress,
    token0,
    token1,
    cpSwapProgram
  );
  const [lpMintAddress] = await getPoolLpMintAddress(
    poolAddress,
    cpSwapProgram
  );
  const [vault0] = await getPoolVaultAddress(
    poolAddress,
    token0,
    cpSwapProgram
  );
  const [vault1] = await getPoolVaultAddress(
    poolAddress,
    token1,
    cpSwapProgram
  );
  const [ownerLpToken] = await PublicKey.findProgramAddress(
    [
      owner.publicKey.toBuffer(),
      TOKEN_PROGRAM_ID.toBuffer(),
      lpMintAddress.toBuffer(),
    ],
    ASSOCIATED_PROGRAM_ID
  );

  const onwerToken0 = getAssociatedTokenAddressSync(
    token0,
    owner.publicKey,
    false,
    token0Program
  );
  const onwerToken1 = getAssociatedTokenAddressSync(
    token1,
    owner.publicKey,
    false,
    token1Program
  );

  const tx = await program.methods
    .proxyDeposit(
      lp_token_amount,
      maximum_token_0_amount,
      maximum_token_1_amount
    )
    .accountsPartial({
      cpSwapProgram: cpSwapProgram,
      owner: owner.publicKey,
      authority: auth,
      poolState: poolAddress,
      ownerLpToken,
      token0Account: onwerToken0,
      token1Account: onwerToken1,
      token0Vault: vault0,
      token1Vault: vault1,
      tokenProgram: TOKEN_PROGRAM_ID,
      tokenProgram2022: TOKEN_2022_PROGRAM_ID,
      vault0Mint: token0,
      vault1Mint: token1,
      lpMint: lpMintAddress,
    })
    .preInstructions([
      ComputeBudgetProgram.setComputeUnitLimit({ units: 400000 }),
    ])
    .rpc(confirmOptions);
  return tx;
}

export async function swap_base_input_accounts(
  owner: Signer,
  configAddress: PublicKey,
  inputToken: PublicKey,
  inputTokenAccount: PublicKey,
  inputTokenProgram: PublicKey,
  outputToken: PublicKey,
  outputTokenProgram: PublicKey
) {
  const [auth] = await getAuthAddress(cpSwapProgram);
  const [poolAddress] = await getPoolAddress(
    configAddress,
    inputToken,
    outputToken,
    cpSwapProgram
  );

  const [inputVault] = await getPoolVaultAddress(
    poolAddress,
    inputToken,
    cpSwapProgram
  );
  const [outputVault] = await getPoolVaultAddress(
    poolAddress,
    outputToken,
    cpSwapProgram
  );

  const outputTokenAccount = getAssociatedTokenAddressSync(
    outputToken,
    owner.publicKey,
    false,
    outputTokenProgram
  );
  const [observationAddress] = await getOrcleAccountAddress(
    poolAddress,
    cpSwapProgram
  );

  let accounts: any = [
    { pubkey: cpSwapProgram, isSigner: false, isWritable: false },
    // { pubkey: owner.publicKey, isSigner: true, isWritable: false },
    { pubkey: auth, isSigner: false, isWritable: false },
    { pubkey: configAddress, isSigner: false, isWritable: false },
    { pubkey: poolAddress, isSigner: false, isWritable: true },
    { pubkey: inputTokenAccount, isSigner: false, isWritable: true },
    { pubkey: outputTokenAccount, isSigner: false, isWritable: true },
    { pubkey: inputVault, isSigner: false, isWritable: true },
    { pubkey: outputVault, isSigner: false, isWritable: true },
    { pubkey: inputTokenProgram, isSigner: false, isWritable: false },
    { pubkey: outputTokenProgram, isSigner: false, isWritable: false },
    { pubkey: inputToken, isSigner: false, isWritable: false },
    { pubkey: outputToken, isSigner: false, isWritable: false },
    { pubkey: observationAddress, isSigner: false, isWritable: true },
  ];

  return accounts;
}
