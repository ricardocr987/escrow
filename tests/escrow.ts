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

  const maker = anchor.web3.Keypair.generate();
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
      maker.publicKey
    );
    makerTokenAccountB = await takerMint.createAssociatedTokenAccount(
      maker.publicKey
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

    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(maker.publicKey, 10000000000),
      "confirmed"
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(taker.publicKey, 10000000000),
      "confirmed"
    );
  });

  it("send to vault", async () => {
    let id = new anchor.BN(0);

    const [escrow, escrowBump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("escrow")),
        id.toBuffer("le", 8)
      ],
      program.programId
    );

    const [vault, vaultBump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        escrow.toBuffer()
      ],
      program.programId
    );

    await program.methods
      .initialize(new anchor.BN(100),new anchor.BN(200), escrowBump, vaultBump, id)
        .accounts({
          escrowAccount: escrow,
          vaultAccount: vault,
          authority: maker.publicKey,
          tokenAccountA: makerTokenAccountA,
          mintA: makerMint.publicKey,
          mintB: takerMint.publicKey,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
          .signers([maker])
            .rpc();
    
    // Check the escrow has the right amount.
    assert.equal(100, (await makerMint.getAccountInfo(vault)).amount.toNumber());
  })

  it("send to vault", async () => {
    let id = new anchor.BN(0);

    const [escrow, escrowBump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("escrow")),
        id.toBuffer("le", 8)
      ],
      program.programId
    );

    const [vault, vaultBump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        escrow.toBuffer()
      ],
      program.programId
    );

    await program.methods
      .initialize(new anchor.BN(100),new anchor.BN(200), escrowBump, vaultBump, id)
        .accounts({
          escrowAccount: escrow,
          vaultAccount: vault,
          authority: maker.publicKey,
          tokenAccountA: makerTokenAccountA,
          mintA: makerMint.publicKey,
          mintB: takerMint.publicKey,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
          .signers([maker])
            .rpc();
    
    // Check the escrow has the right amount.
    assert.equal(100, (await makerMint.getAccountInfo(vault)).amount.toNumber());

  })

  it("send to vault and cancel", async () => {
    let id = new anchor.BN(1);

    const [escrow, escrowBump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("escrow")),
        id.toBuffer("le", 8)
      ],
      program.programId
    );

    const [vault, vaultBump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        escrow.toBuffer()
      ],
      program.programId
    );

    const startingTokenBalance = (await makerMint.getAccountInfo(makerTokenAccountA)).amount.toNumber();

    await program.methods
      .initialize(new anchor.BN(100),new anchor.BN(200), escrowBump, vaultBump, id)
        .accounts({
          escrowAccount: escrow,
          vaultAccount: vault,
          authority: maker.publicKey,
          tokenAccountA: makerTokenAccountA,
          mintA: makerMint.publicKey,
          mintB: takerMint.publicKey,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
          .signers([maker])
            .rpc();
    
    // Check the escrow has the right amount.
    assert.equal(100, (await makerMint.getAccountInfo(vault)).amount.toNumber());

    await program.methods
    .cancel()
      .accounts({
        escrowAccount: escrow,
        vaultAccount: vault,
        authority: program.provider.wallet.publicKey,
        tokenAccountA: makerTokenAccountA,
        tokenProgram: spl.TOKEN_PROGRAM_ID
      });

      console.log((await makerMint.getAccountInfo(vault)).amount.toNumber());
      
    // The underlying escrow account got closed when the offer got cancelled.
    assert.equal(null, await program.provider.connection.getAccountInfo(escrow));
    // The vault account got closed when the offer got cancelled.
    assert.equal(null, await program.provider.connection.getAccountInfo(vault));

    // The offer maker got their tokens back.
    assert.equal(startingTokenBalance, (await makerMint.getAccountInfo(makerTokenAccountA)).amount.toNumber())

    // See what happens if we accept despite already canceling...
    try {
      await program.methods
        .exchange()
          .accounts({
            escrowAccount: escrow,
            vaultAccount: vault,
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
      assert.fail("Accepting a previously-cancelled offer should have failed");
    } catch (e) {
      // The offer account got closed when we accepted the offer, so trying to
      // use it again results in "not owned by the program" error (as expected).
      assert.equal(0xa7, e.code);
    }
  });
  it("Error: exchange with the wrong kind of tokens", async () => {
    let id = new anchor.BN(2);

    const [escrow, escrowBump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("escrow")),
        id.toBuffer("le", 8)
      ],
      program.programId
    );

    const [vault, vaultBump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        escrow.toBuffer()
      ],
      program.programId
    );

    await program.methods
    .initialize(new anchor.BN(100),new anchor.BN(200), escrowBump, vaultBump, id)
    .accounts({
      escrowAccount: escrow,
      vaultAccount: vault,
      authority: maker.publicKey,
      tokenAccountA: makerTokenAccountA,
      mintA: makerMint.publicKey,
      mintB: takerMint.publicKey,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      tokenProgram: spl.TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
      .signers([maker])
        .rpc();

    // Check the escrow has the right amount.
    assert.equal(100, (await makerMint.getAccountInfo(vault)).amount.toNumber());

    try {
      await program.methods
      .exchange()
        .accounts({
          escrowAccount: escrow,
          vaultAccount: vault,
          maker: program.provider.wallet.publicKey,
          authority: taker.publicKey,
          makerTokenAccountB: makerTokenAccountB,
          takerTokenAccountB: takerTokenAccountB,
          takerTokenAccountA: offerTakersRandomOtherTokens,
          takerMint: takerMint.publicKey,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
          .signers([taker])
            .rpc();
      assert.fail("Shouldn't have been able to accept an offer with the wrong type of tokens");
    } catch (e) {
      // Should trigger a constraint
      assert.equal(0x8f, e.code);
    }

    // The underlying offer account got closed when the offer got cancelled.
    assert.notEqual(null, await program.provider.connection.getAccountInfo(escrow));
    // The escrow account got closed when the offer got accepted.
    assert.notEqual(null, await program.provider.connection.getAccountInfo(vault));
  });

  it("Error: won't let you accept an offer with the wrong amount", async () => {
    let id = new anchor.BN(3);

    const [escrow, escrowBump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("escrow")),
        id.toBuffer("le", 8)
      ],
      program.programId
    );

    const [vault, vaultBump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        escrow.toBuffer()
      ],
      program.programId
    );

    await program.methods
    .initialize(new anchor.BN(100),new anchor.BN(200), escrowBump, vaultBump, id)
    .accounts({
      escrowAccount: escrow,
      vaultAccount: vault,
      authority: maker.publicKey,
      tokenAccountA: makerTokenAccountA,
      mintA: makerMint.publicKey,
      mintB: takerMint.publicKey,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      tokenProgram: spl.TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
      .signers([maker])
        .rpc();

    // Check the escrow has the right amount.
    assert.equal(100, (await makerMint.getAccountInfo(vault)).amount.toNumber());

    try {
      await program.methods
      .exchange()
        .accounts({
          escrowAccount: escrow,
          vaultAccount: vault,
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
      
      assert.fail("Shouldn't have been able to accept an offer with too few tokens");
    } catch (e) {
      assert.ok(e.logs.some(log => log.includes("insufficient funds")));
    }

    // The underlying offer account got closed when the offer got cancelled.
    assert.notEqual(null, await program.provider.connection.getAccountInfo(escrow));
    // The escrow account got closed when the offer got accepted.
    assert.notEqual(null, await program.provider.connection.getAccountInfo(vault));
  });

  it("Error: won't let you accept an offer with a token account that doesn't belong to the maker", async () => {
    let id = new anchor.BN(4);

    const [escrow, escrowBump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("escrow")),
        id.toBuffer("le", 8)
      ],
      program.programId
    );

    const [vault, vaultBump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        escrow.toBuffer()
      ],
      program.programId
    );

    await program.methods
    .initialize(new anchor.BN(100),new anchor.BN(200), escrowBump, vaultBump, id)
    .accounts({
      escrowAccount: escrow,
      vaultAccount: vault,
      authority: maker.publicKey,
      tokenAccountA: makerTokenAccountA,
      mintA: makerMint.publicKey,
      mintB: takerMint.publicKey,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      tokenProgram: spl.TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
      .signers([maker])
        .rpc();

    // Check the escrow has the right amount.
    assert.equal(100, (await makerMint.getAccountInfo(vault)).amount.toNumber());

    try {
      await program.methods
      .exchange()
        .accounts({
          escrowAccount: escrow,
          vaultAccount: vault,
          maker: program.provider.wallet.publicKey,
          authority: taker.publicKey,
          makerTokenAccountB: makerTokenAccountB,
          takerTokenAccountB: takerTokenAccountB,
          takerTokenAccountA: offerTakersRandomOtherTokens,
          takerMint: takerMint.publicKey,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
          .signers([taker])
            .rpc();
      assert.fail("Shouldn't have been able to accept an offer with a token account that doesn't belong to the maker");
    } catch (e) {
      // Should trigger an associated token constraint
      assert.equal(0x95, e.code);
    }

    // The underlying offer account got closed when the offer got cancelled.
    assert.notEqual(null, await program.provider.connection.getAccountInfo(escrow));
    // The escrow account got closed when the offer got accepted.
    assert.notEqual(null, await program.provider.connection.getAccountInfo(vault));
  });
});
