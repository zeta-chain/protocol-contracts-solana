import * as anchor from "@coral-xyz/anchor";
import { Connected } from "../target/types/connected";

(async () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Connected as anchor.Program<Connected>;

  const tx = await program.methods
    .initialize()
    .accounts({
      signer: provider.wallet.publicKey,
    })
    .rpc();

  console.log("Initialize transaction sent");
  console.log("Tx Signature:", tx);
})();
