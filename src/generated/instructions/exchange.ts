/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as splToken from '@solana/spl-token'
import * as beet from '@metaplex-foundation/beet'
import * as web3 from '@solana/web3.js'

/**
 * @category Instructions
 * @category Exchange
 * @category generated
 */
const exchangeStruct = new beet.BeetArgsStruct<{
  instructionDiscriminator: number[] /* size: 8 */
}>(
  [['instructionDiscriminator', beet.uniformFixedSizeArray(beet.u8, 8)]],
  'ExchangeInstructionArgs'
)
/**
 * Accounts required by the _exchange_ instruction
 * @category Instructions
 * @category Exchange
 * @category generated
 */
export type ExchangeInstructionAccounts = {
  escrowAccount: web3.PublicKey
  vaultAccount: web3.PublicKey
  maker: web3.PublicKey
  authority: web3.PublicKey
  makerTokenAccountB: web3.PublicKey
  takerTokenAccountB: web3.PublicKey
  takerTokenAccountA: web3.PublicKey
  takerMint: web3.PublicKey
}

const exchangeInstructionDiscriminator = [47, 3, 27, 97, 215, 236, 219, 144]

/**
 * Creates a _Exchange_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 *
 * @category Instructions
 * @category Exchange
 * @category generated
 */
export function createExchangeInstruction(
  accounts: ExchangeInstructionAccounts
) {
  const {
    escrowAccount,
    vaultAccount,
    maker,
    authority,
    makerTokenAccountB,
    takerTokenAccountB,
    takerTokenAccountA,
    takerMint,
  } = accounts

  const [data] = exchangeStruct.serialize({
    instructionDiscriminator: exchangeInstructionDiscriminator,
  })
  const keys: web3.AccountMeta[] = [
    {
      pubkey: escrowAccount,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: vaultAccount,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: maker,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: authority,
      isWritable: true,
      isSigner: true,
    },
    {
      pubkey: makerTokenAccountB,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: takerTokenAccountB,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: takerTokenAccountA,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: takerMint,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: splToken.TOKEN_PROGRAM_ID,
      isWritable: false,
      isSigner: false,
    },
  ]

  const ix = new web3.TransactionInstruction({
    programId: new web3.PublicKey(
      'D7ko992PKYLDKFy3fWCQsePvWF3Z7CmvoDHnViGf8bfm'
    ),
    keys,
    data,
  })
  return ix
}
