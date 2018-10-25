use std::collections::HashSet;

use bitcoin_amount::Amount;
use bitcoinconsensus;
use rbitcoin::blockdata::opcodes;
use rbitcoin::blockdata::script::Builder;
use rbitcoin::blockdata::transaction::{OutPoint, Transaction, TxIn, TxOut};
use rbitcoin::consensus::encode::{deserialize, serialize};
use rbitcoin::util::hash::Sha256dHash;
use rbitcoin::util::psbt;

use common::*;
use protos;
use utils;

/// An internal type used to represent a transaction output with extra metadata.
#[derive(Debug)]
pub struct UTXO {
	pub point: OutPoint,

	pub psbt_input: psbt::Input,

	// meta-information: This information is not critical for proof verification.
	pub block_number: u32,
	pub block_hash: Option<Sha256dHash>,
}

impl UTXO {
	/// Get the tx output for this UTXO.
	pub fn txout(&self) -> &TxOut {
		if let Some(ref txout) = self.psbt_input.witness_utxo {
			txout
		} else if let Some(ref tx) = self.psbt_input.non_witness_utxo {
			&tx.output[self.point.vout as usize]
		} else {
			panic!("Incorrect PSBT input data for UTXO {}", self.point);
		}
	}

	/// Get the output amount as an Amount object.
	pub fn value(&self) -> Amount {
		Amount::from_sat(self.txout().value as i64)
	}

	/// Returns whether or not the UTXO is using segregated witness.
	pub fn is_witness(&self) -> bool {
		let ref script = self.txout().script_pubkey;
		script.is_v0_p2wsh() || script.is_v0_p2wpkh()
	}
}

impl From<protos::UTXO> for UTXO {
	fn from(o: protos::UTXO) -> Self {
		UTXO {
			point: OutPoint {
				txid: o.get_txid().into(),
				vout: o.get_vout(),
			},
			psbt_input: deserialize(o.get_psbt_input())
				.expect("corrupt PSBT input in reserve file"),
			block_number: o.get_block_number(),
			block_hash: if o.get_block_hash().len() != 0 {
				Some(o.get_block_hash().into())
			} else {
				None
			},
		}
	}
}

impl Into<protos::UTXO> for UTXO {
	fn into(self) -> protos::UTXO {
		let mut p = protos::UTXO::new();
		p.set_txid(self.point.txid.as_bytes()[..].into());
		p.set_vout(self.point.vout);
		p.set_psbt_input(serialize(&self.psbt_input));
		p.set_block_number(self.block_number);
		if let Some(hash) = self.block_hash {
			p.set_block_hash(hash.as_bytes()[..].into());
		}
		p
	}
}

/// Generate the challenge input based on the challenge string.
/// The input is created by using the SHA-256 hash of the challenge as the prevout hash.
pub fn challenge_txin(challenge: &str) -> TxIn {
	let challenge_hash = utils::sha256(challenge.as_bytes());
	TxIn {
		previous_output: OutPoint {
			txid: challenge_hash[..].into(),
			vout: 0,
		},
		sequence: 0xFFFFFFFF,
		script_sig: Builder::new().into_script(),
		witness: vec![],
	}
}

#[derive(Debug)]
pub struct Proof {
	pub id: String,
	pub status: Proof_Status,

	pub proof_tx: Option<Transaction>,
	pub utxos: Vec<UTXO>,
	pub psbt: Option<psbt::PartiallySignedTransaction>,
}

impl From<protos::Proof> for Proof {
	fn from(p: protos::Proof) -> Self {
		Proof {
			id: p.id.into(),
			status: p.status,
			proof_tx: if p.proof_tx.len() > 0 {
				Some(deserialize(&p.proof_tx).expect("corrupt proof tx"))
			} else {
				None
			},
			utxos: p
				.utxos
				.into_vec()
				.into_iter()
				.map(|u| {
					let i: UTXO = u.into();
					i
				}).collect(),
			psbt: if p.psbt.len() > 0 {
				Some(deserialize(&p.psbt).expect("corrupt PSBT in proof"))
			} else {
				None
			},
		}
	}
}

impl Into<protos::Proof> for Proof {
	fn into(self) -> protos::Proof {
		let mut p = protos::Proof::new();
		p.set_id(self.id.into());
		p.set_status(self.status);
		if let Some(proof_tx) = self.proof_tx {
			p.set_proof_tx(serialize(&proof_tx));
		}
		p.set_utxos(
			self.utxos
				.into_iter()
				.map(|u| {
					let i: protos::UTXO = u.into();
					i
				}).collect(),
		);
		if let Some(psbt) = self.psbt {
			p.set_psbt(serialize(&psbt));
		}
		p
	}
}

impl Proof {
	pub fn new(id: String, status: Proof_Status) -> Proof {
		Proof {
			id: id,
			status: status,
			proof_tx: None,
			utxos: vec![],
			psbt: None,
		}
	}

	/// Advance the proof to the SIGNING state by constructing a PSBT transaction to be signed.
	pub fn start_signing(&mut self, challenge: &str) {
		let mut tx_inputs = Vec::new();
		let mut psbt_inputs = Vec::new();

		// Add the challenge txin.
		let challenge_txin = challenge_txin(challenge);
		tx_inputs.push(challenge_txin);
		psbt_inputs.push(psbt::Input {
			witness_utxo: Some(TxOut {
				value: 0,
				script_pubkey: Builder::new().push_opcode(opcodes::OP_TRUE).into_script(),
			}),
			witness_script: Some(Builder::new().into_script()),
			final_script_sig: Some(Builder::new().into_script()),
			..Default::default()
		});

		// Then add all proof UTXOs as inputs.
		let mut total_amount = 0;
		for utxo in self.utxos.iter() {
			tx_inputs.push(TxIn {
				previous_output: utxo.point,
				sequence: 0xFFFFFFFF,
				script_sig: Builder::new().into_script(),
				witness: Vec::new(),
			});
			psbt_inputs.push(utxo.psbt_input.clone());
			total_amount += utxo.value().into_inner();
		}

		// Construct the tx and psbt tx.
		let tx = Transaction {
			version: 1,
			lock_time: 0xffffffff, // Max time in the future. 2106-02-07 06:28:15
			input: tx_inputs,
			output: vec![TxOut {
				value: total_amount as u64,
				script_pubkey: Builder::new().push_opcode(opcodes::OP_FALSE).into_script(),
			}],
		};
		let mut psbt = psbt::PartiallySignedTransaction::from_unsigned_tx(tx)
			.expect("error constructing PSBT from unsigned tx");
		psbt.inputs = psbt_inputs;
		// We can leave the one psbt output empty.

		self.psbt = Some(psbt);
		self.status = Proof_Status::SIGNING;
	}

	/// Return all the outpoins this proof is spending.
	pub fn spending_utxos(&self) -> HashSet<OutPoint> {
		let mut set = HashSet::new();
		for input in self.proof_tx.as_ref().unwrap().input.iter() {
			if !set.insert(input.previous_output) {
				panic!("Proof '{}' is spending UTXO {} twice!", self.id, input.previous_output);
			}
		}
		set
	}

	pub fn verify(&self, challenge: &str, prevouts: Vec<TxOut>) {
		let tx = self.proof_tx.as_ref().expect("proof in wrong state");
		// Proof tx must have exactly 1 output and more than 1 inputs.
		if tx.output.len() != 1 {
			panic!(
				"Proof tx for proof '{}' must have exactly 1 output (has {})!",
				self.id,
				tx.output.len()
			);
		}
		if tx.input.len() <= 1 {
			panic!(
				"Proof tx for proof '{}' must have more than one inputs (has {})!",
				self.id,
				tx.input.len()
			);
		}
		if prevouts.len() != tx.input.len() - 1 {
			panic!("Wrong amount of prevouts provided");
		}

		// First check the challenge input.
		let challenge_txin = challenge_txin(challenge);
		if tx.input[0].previous_output != challenge_txin.previous_output {
			panic!("Challenge for proof '{}' is incorrect", self.id);
		}

		// Verify other inputs against prevouts and calculate the amount.
		let serialized_tx = serialize(tx);
		let mut total_amount = 0;
		for (idx, txout) in prevouts.into_iter().enumerate() {
			// Verify the script execution of the input.
			bitcoinconsensus::verify(
				txout.script_pubkey.to_bytes().as_slice(),
				txout.value,
				&serialized_tx,
				idx + 1, // skipped the challenge input
			).expect(&format!(
				"script verification of input #{} of proof tx '{}' failed",
				idx, self.id
			));

			total_amount += txout.value;
		}

		// Verify the amounts.  They must match exactly; no fee.
		let output_amount = tx.output[0].value;
		if total_amount != output_amount {
			panic!(
				"Amounts of proof '{}' do not add up! Inputs: {}; Outputs: {}.",
				self.id, total_amount, output_amount,
			);
		}

		println!("Successfully verified proof '{}' for {} satoshis.", self.id, total_amount);
	}
}
