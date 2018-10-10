reserves
========


reserves is a tool for generating and verifying proof-of-reserves for funds
in the Bitcoin network.


# Features

It currently has the following features:

- use of a single "proof file" that can include several seperate proofs to 
	ease the use of distinct wallets

- add a proof challenge to prevent proofs to be reused or exchanged

- two-step procedure to ease use with hardware wallet or HSMs: first 
	collecting UTXOs to be bundled, then signing the proof

- proofs are made at a specific block number and can be verified even if the
	funds moved after the point of proving

- relying on existing standards: Final proofs are unspendable but valid
	Bitcoin transactions and in-progress proofs are kept in PSBT format to ease
	integration with hardware wallets.


# How it works

For every proof-of-reserves, a Bitcoin transaction will be generated.  This
transaction will be invalidated so that it cannot be broadcast to the Bitcoin
network.  This is done by adding an input that refers to a non-existing UTXO.

The remainder of the transaction consists of UTXOs owned by the proving party
and a single output with the sum of the values of all the UTXOs in the inputs.
The prover signs this transaction to prove that it can spend the UTXOs.

Since the transaction contains a non-existing input, the provers inputs cannot
actually be spent, but the signatures on the inputs can be verified as if the
transaction did not contain the non-existing input to verify the proof.



# Usage

The proof file if formatted with protobuf using the spec in the `protos/`
folder.  The file format can be reused by in-house applications if preferred.


## Some example usage


### init: initialize a proof file

```
$ reserves init -f reserves.proof --challenge "Blockstream August 2018" \
	--block-number 12345
```
Creates a proof file `reserves.proof` (this is also the default if `-f` is
ommitted) with the given challenge and block number.

### inspect: inspect the contents of a proof file

```
$ reserves inspect -f reserves.proof
```

### add-utxos: add UTXOs to a proof

Add UTXOs to a proof using one of the provided sources.  Currently the only
available source is the Bitcoin Core wallet.

```
$ reserves add-utxos --bitcoind http://localhost:8332 \
	--bitcoind-user rpcuser --bitcoind-pass rpcpass
```

### sign: sign a proof

Once all desired UTXOs for a proof are collected, the prover can sign the proof
transaction.  Currently the only supported wallet for signing is Bitcoin Core.

```
$ reserves sign --bitcoind http://localhost:8332 \
	--bitcoind-user rpcuser --bitcoind-pass rpcpass
```

### verify: verify a proof

This will also verify the validity of the UTXOs, thus a bitcoind reference is
required for this call.

```
$ reserves verify -f reserves.proof --bitcoind http://localhost:8332 \
	--bitcoind-user rpcuser --bitcoind-pass rpcpass
```


# Future Work

- Support more UTXO sources (Elecrum, manual entry, ...).

- Support more wallets for signing: Ledger, Trezor, ...

- Support Liquid. If possible BTC-only first and then general CA support.

- Add privacy using the Provisions scheme by Benedikt Bunz.  This gets
	especially interesting once Schnorr usage is more common.  Potentially also
	add proof-of-liabilities support.
