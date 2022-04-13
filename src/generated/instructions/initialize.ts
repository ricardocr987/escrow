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
 * @category Initialize
 * @category generated
 */
export type InitializeInstructionArgs = {
  amountA: beet.bignum
  amountB: beet.bignum
  escrowBump: number
  vaultBump: number
  id: beet.bignum
}
/**
 * @category Instructions
 * @category Initialize
 * @category generated
 */
const initializeStruct = new beet.BeetArgsStruct<
  InitializeInstructionArgs & {
    instructionDiscriminator: number[] /* size: 8 */
  }
>(
  [
    ['instructionDiscriminator', beet.uniformFixedSizeArray(beet.u8, 8)],
    ['amountA', beet.u64],
    ['amountB', beet.u64],
    ['escrowBump', beet.u8],
    ['vaultBump', beet.u8],
    ['id', beet.u64],
  ],
  'InitializeInstructionArgs'
)
/**
 * Accounts required by the _initialize_ instruction
 * @category Instructions
 * @category Initialize
 * @category generated
 */
export type InitializeInstructionAccounts = {
  escrowAccount: web3.PublicKey
  vaultAccount: web3.PublicKey
  authority: web3.PublicKey
  tokenAccountA: web3.PublicKey
  mintA: web3.PublicKey
  mintB: web3.PublicKey
}

const initializeInstructionDiscriminator = [
  175, 175, 109, 31, 13, 152, 155, 237,
]

/**
 * Creates a _Initialize_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @param args to provide as instruction data to the program
 *
 * @category Instructions
 * @category Initialize
 * @category generated
 */
export function createInitializeInstruction(
  accounts: InitializeInstructionAccounts,
  args: InitializeInstructionArgs
) {
  const {
    escrowAccount,
    vaultAccount,
    authority,
    tokenAccountA,
    mintA,
    mintB,
  } = accounts

  const [data] = initializeStruct.serialize({
    instructionDiscriminator: initializeInstructionDiscriminator,
    ...args,
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
      pubkey: authority,
      isWritable: true,
      isSigner: true,
    },
    {
      pubkey: tokenAccountA,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: mintA,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: mintB,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: web3.SYSVAR_RENT_PUBKEY,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: splToken.TOKEN_PROGRAM_ID,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: web3.SystemProgram.programId,
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