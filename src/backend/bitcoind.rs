use bitcoincore_rpc as rpc;
use bitcoincore_rpc::GetTransaction;
use bitcoincore_rpc::Queryable;
use clap;
use rbitcoin::blockdata::opcodes;
use rbitcoin::blockdata::script::Builder;
use rbitcoin::consensus::encode::serialize;
use rbitcoin::util::hash::BitcoinHash;
use rbitcoin::util::psbt;
use rbitcoin::{Block, OutPoint, Transaction, TxOut};

use bitcoin::*;

pub fn args<'a>() -> Vec<clap::Arg<'a, 'a>> {
	vec![
		clap::Arg::with_name("bitcoind")
			.long("bitcoind")
			.help("the RPC endpoint for bitcoind")
			.takes_value(true),
		clap::Arg::with_name("bitcoind-user")
			.long("bitcoind-user")
			.help("the RPC user for bitcoind")
			.takes_value(true),
		clap::Arg::with_name("bitcoind-pass")
			.long("bitcoind-pass")
			.help("the RPC pass for bitcoind")
			.takes_value(true),
	]
}

pub struct Backend(rpc::Client);

impl Backend {
	pub fn load<'a>(matches: &'a clap::ArgMatches) -> Option<Self> {
		match matches.value_of("bitcoind") {
			None => None,
			Some(endpoint) => Some(Backend(rpc::Client::new(
				endpoint.to_string(),
				match matches.value_of("bitcoind-user") {
					Some(v) => Some(v.to_string()),
					None => None,
				},
				match matches.value_of("bitcoind-pass") {
					Some(v) => Some(v.to_string()),
					None => None,
				},
			))),
		}
	}

	/// Fetch unspent outputs from the node's wallet.
	pub fn fetch_utxos(&mut self) -> Vec<UTXO> {
		let mut utxos = Vec::new();

		let unspents = self
			.0
			.list_unspent(Some(6), None, None, None, None)
			.expect("failed to fetch utxos from bitcoind");
		for unspent in unspents.into_iter() {
			if !unspent.spendable {
				continue;
			}

			// Fetch tx and block info.
			let tx_info = self
				.0
				.get_raw_transaction_verbose(&unspent.txid, None)
				.expect("error retrieving raw tx from bitcoind");
			let block_info = self
				.0
				.get_block_header_verbose(&tx_info.blockhash)
				.expect("error loading block header from bitcoind");

			let tx = tx_info.transaction().expect(&format!("failed to decode tx {}", unspent.txid));
			let txout = tx.output.get(unspent.vout as usize).expect("unspent vout doesn't exist");

			let mut psbt_input: psbt::Input = Default::default();
			psbt_input.non_witness_utxo = Some(tx.clone());
			psbt_input.witness_utxo = Some(txout.clone());
			psbt_input.redeem_script = unspent.redeem_script;

			utxos.push(UTXO {
				point: OutPoint {
					txid: unspent.txid,
					vout: unspent.vout,
				},
				psbt_input: psbt_input,
				block_number: block_info.height as u32,
				block_hash: Some(tx_info.blockhash),
			});
		}
		utxos
	}

	/// Ask bitcoind to sign the given tx.
	pub fn sign_tx(&mut self, tx: Transaction) -> Transaction {
		// Encode tx to pass to bitcoind.
		let raw_tx = serialize(&tx);

		// Construct fictive challenge UTXO to pass to signer.
		let challenge_txin = &tx.input[0];
		let challenge_utxo = rpc::json::UTXO {
			txid: &challenge_txin.previous_output.txid,
			vout: challenge_txin.previous_output.vout,
			script_pub_key: &Builder::new().push_opcode(opcodes::OP_TRUE).into_script(),
			redeem_script: &Builder::new().into_script(),
		};

		let signed_res = self
			.0
			.sign_raw_transaction_with_wallet(
				raw_tx.as_slice().into(),
				Some(&[challenge_utxo]),
				None,
			).expect("error calling signrawtransaction");
		if !signed_res.errors.is_empty() {
			for e in signed_res.errors.iter() {
				println!("signing error for input {}: {}", e.vout, e.error);
			}
			panic!("failed to sign proof tx")
		}

		// Decode resulting tx.
		signed_res.transaction().expect("failed to parse signed transaction from bitcoind")
	}

	/// Fetch the previous outpoints for the inputs of the proof tx.  We do this to verify they
	/// all existed at the given block number.
	pub fn fetch_proof_prevouts(&mut self, proof: &Proof, proof_block_number: u32) -> Vec<TxOut> {
		let mut prevouts = Vec::new();
		for (idx, input) in proof.proof_tx.as_ref().unwrap().input.iter().enumerate() {
			// Skip the challenge input.
			if idx == 0 {
				continue;
			}

			// We have two possible scenarios here:
			// 1. the output is currently unspent: we can easily fetch it
			// 2. the output is no longer unspent: we have to find the output in the blockchain

			// Check if the output is unspent.
			let unspent = self
				.0
				.get_tx_out(&input.previous_output.txid, input.previous_output.vout, Some(false))
				.expect(&format!(
					"fetch txout {} (input #{} of proof '{}')",
					input.previous_output, idx, proof.id,
				));
			if let Some(unspent) = unspent {
				// Verify the block number.
				let block_hash = self
					.0
					.get_raw_transaction_verbose(&input.previous_output.txid, None)
					.expect(&format!(
						"error fetching transaction details for txid {} for proof '{}'",
						input.previous_output.txid, proof.id
					)).blockhash;
				let block_number = self
					.0
					.get_block_header_verbose(&block_hash)
					.expect(&format!("fetching block header for {}", unspent.bestblock))
					.height as u32;
				if block_number > proof_block_number {
					panic!(
						"Input {} of proof '{}' was not valid at block {} (bestblock {})",
						input.previous_output, proof.id, proof_block_number, block_number
					);
				}

				prevouts.push(TxOut {
					value: unspent.value.into_inner() as u64,
					script_pubkey: unspent
						.script_pub_key
						.script()
						.expect("corrupt script from RPC"),
				});
				continue;
			};

			// The output is no longer unspent. We have to verify if it was valid before the block
			// number in the proof file.

			// If the block number is provided in the proof file
			let existing = proof.utxos.iter().find(|u| u.point == input.previous_output);
			if let Some(existing) = existing {
				let mut block_hash = existing.block_hash;
				// Get block hash from number.
				if existing.block_hash.is_none() && existing.block_number != 0 {
					block_hash = Some(self.0.get_block_hash(existing.block_number).expect(
						&format!("fetching block hash for block number {}", existing.block_number),
					));
				}

				if let Some(block_hash) = block_hash {
					let block = Block::query(&mut self.0, &block_hash)
						.expect(&format!("fetching block number {}", existing.block_number));
					let found = block
						.txdata
						.into_iter()
						.find(|tx| tx.bitcoin_hash() == input.previous_output.txid);
					if let Some(tx) = found {
						let out =
							tx.output.get(input.previous_output.vout as usize).expect(&format!(
								"outpoint of tx #{} from proof tx '{}' contains non-existent vout {}",
								idx,
								proof.id,
								input.previous_output.vout,
							));
						prevouts.push(out.clone());
						continue;
					}
				}
			}

			//TODO(stevenroose) implement searching for the tx from the proof blocknumber backwards
			panic!("Cannot find output for input #{} of proof tx '{}'!", idx, proof.id);
		}
		prevouts
	}
}
