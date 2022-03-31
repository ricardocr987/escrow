import { Escrow } from "../target/types/escrow";
import * as anchor from '@project-serum/anchor';
import * as spl from '@solana/spl-token';
import { Program } from '@project-serum/anchor';
import * as assert from 'assert';
import { Connection, Keypair, PublicKey, Transaction } from '@solana/web3.js';

interface Wallet {
  signTransaction(tx: Transaction): Promise<Transaction>;
  signAllTransactions(txs: Transaction[]): Promise<Transaction[]>;
  publicKey: PublicKey;
}

class NodeWallet implements Wallet {
  constructor(readonly payer: Keypair) {}

  static local(): NodeWallet {
    const process = require("process");
    const payer = Keypair.fromSecretKey(
      Buffer.from(
        JSON.parse(
          require("fs").readFileSync(process.env.ANCHOR_WALLET, {
            encoding: "utf-8",
          })
        )
      )
    );
    return new NodeWallet(payer);
  }

  async signTransaction(tx: Transaction): Promise<Transaction> {
    tx.partialSign(this.payer);
    return tx;
  }

  async signAllTransactions(txs: Transaction[]): Promise<Transaction[]> {
    return txs.map((t) => {
      t.partialSign(this.payer);
      return t;
    });
  }

  get publicKey(): PublicKey {
    return this.payer.publicKey;
  }
}


describe("escrow", () => {
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Escrow as Program<Escrow>;

  let makerMint: spl.Token;
  let takerMint: spl.Token;
  let randomOtherMint: spl.Token;
  let makerTokenAccountA: anchor.web3.PublicKey;
  let makerTokenAccountB: anchor.web3.PublicKey;
  let takerTokenAccountB: anchor.web3.PublicKey;
  let takerTokenAccountA: anchor.web3.PublicKey;
  let offerTakersRandomOtherTokens: anchor.web3.PublicKey;
  let hackersTakerTokens: anchor.web3.PublicKey;
  const taker = anchor.web3.Keypair.generate();
  const hacker = anchor.web3.Keypair.generate();

  before(async () => {
    const wallet = program.provider.wallet as NodeWallet;
    makerMint = await spl.Token.createMint(
      program.provider.connection,
      wallet.payer,
      wallet.publicKey,
      wallet.publicKey,
      0,
      spl.TOKEN_PROGRAM_ID
    );
    takerMint = await spl.Token.createMint(
      program.provider.connection,
      wallet.payer,
      wallet.publicKey,
      wallet.publicKey,
      0,
      spl.TOKEN_PROGRAM_ID
    );
    randomOtherMint = await spl.Token.createMint(
      program.provider.connection,
      wallet.payer,
      wallet.publicKey,
      wallet.publicKey,
      0,
      spl.TOKEN_PROGRAM_ID
    );
    makerTokenAccountA = await makerMint.createAssociatedTokenAccount(
      program.provider.wallet.publicKey
    );
    makerTokenAccountB = await takerMint.createAssociatedTokenAccount(
      program.provider.wallet.publicKey
    );
    takerTokenAccountA = await makerMint.createAssociatedTokenAccount(
      taker.publicKey
    );
    takerTokenAccountB = await takerMint.createAssociatedTokenAccount(
      taker.publicKey
    );
    offerTakersRandomOtherTokens = await randomOtherMint.createAssociatedTokenAccount(
      taker.publicKey
    );
    hackersTakerTokens = await takerMint.createAssociatedTokenAccount(
      hacker.publicKey
    );

    await makerMint.mintTo(makerTokenAccountA, program.provider.wallet.publicKey, [], 1000);
    await takerMint.mintTo(takerTokenAccountB, program.provider.wallet.publicKey, [], 1000);
  });

  it("lets you make and accept offers", async () => {
    const escrow = anchor.web3.Keypair.generate();

    const [escrowedMakerTokens, escrowedMakerTokensBump] = await anchor.web3.PublicKey.findProgramAddress(
      [escrow.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .initialize(escrowedMakerTokensBump, new anchor.BN(100),new anchor.BN(200))
      .accounts({
        escrowAccount: escrow.publicKey,
        authority: program.provider.wallet.publicKey,
        makerTokenAccountA: makerTokenAccountA,
        vaultAccount: escrowedMakerTokens,
        makerMint: makerMint.publicKey,
        takerMint: takerMint.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([escrow])
      .rpc();

    // Check the escrow has the right amount.
    assert.equal(100, (await makerMint.getAccountInfo(escrowedMakerTokens)).amount.toNumber());
  
    await program.methods
      .exchange()
      .accounts({
        escrowAccount: escrow.publicKey,
        vaultAccount: escrowedMakerTokens,
        maker: program.provider.wallet.publicKey,
        authority: taker.publicKey,
        makerTokenAccountB: makerTokenAccountB,
        takerTokenAccountB: takerTokenAccountB,
        takerTokenAccountA: takerTokenAccountA,
        takerMint: takerMint.publicKey,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
      })
      .signers([taker])
      .rpc();

    assert.equal(100, (await makerMint.getAccountInfo(takerTokenAccountA)).amount.toNumber());
    assert.equal(200, (await takerMint.getAccountInfo(makerTokenAccountB)).amount.toNumber());

    // The underlying offer account got closed when the offer got cancelled.
    assert.equal(null, await program.provider.connection.getAccountInfo(escrow.publicKey));
    // The escrow account got closed when the offer got accepted.
    assert.equal(null, await program.provider.connection.getAccountInfo(escrowedMakerTokens));
  });
});