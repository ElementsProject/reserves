use clap;
use hex;
use rbitcoin::consensus::encode as btc_encode;

use bitcoin;
use common::*;
use context;
use utils;

/// Create the verify subcommand.
pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	clap::SubCommand::with_name("inspect").about("inspect the contents of the proof file")
}

fn print_outputs(pre: &str, proof: &bitcoin::Proof) {
	let nb_utxos = proof.utxos.len();
	println!("{}{} UTXOs:", pre, nb_utxos);
	for (idx, u) in proof.utxos.iter().enumerate() {
		println!("{}  outpoint: {}", pre, u.point);
		println!("{}  value: {:?}", pre, u.value()); //TODO(stevenroose) pretty print
		info!("PSBT input: {:?}", u.psbt_input);
		println!("{}  block number: {}", pre, u.block_number);
		println!(
			"{}  block hash: {}",
			pre,
			u.block_hash.map(|h| h.be_hex_string()).or_else(|| Some("unknown".to_owned())).unwrap()
		);

		if idx != nb_utxos - 1 {
			println!("");
		}
	}
}

/// Execute the verify command.
pub fn execute(ctx: &mut context::Ctx) {
	let pf = ctx.load_proof_file();

	println!("version: {}", pf.version);
	println!("network: {}", utils::network_name(pf.network));
	println!("challenge: {}", pf.challenge);
	println!("block number: {}", pf.block_number);

	// Print all proofs:
	let nb_proofs = pf.proofs.len();
	println!("{} proof(s):", nb_proofs);
	for (idx, proof) in pf.proofs.into_iter().enumerate() {
		println!("  id: {}", proof.id);
		println!("  status: {:?}", proof.status);

		match proof.status {
			Proof_Status::UNDEFINED => {}
			Proof_Status::FINAL => {
				let amount =
					proof.proof_tx.as_ref().unwrap().output.iter().fold(0, |a, o| a + o.value);
				println!("  amount: {} satoshis", amount);
				println!("  raw proof tx: {}", hex::encode(btc_encode::serialize(&proof.proof_tx)));
				info!("decoded proof tx: {:?}", proof.proof_tx);
				print_outputs("  ", &proof);
			}
			Proof_Status::GATHERING_UTXOS => {
				print_outputs("  ", &proof);
			}
			Proof_Status::SIGNING => {
				//TODO(stevenroose)
				print_outputs("  ", &proof);
			}
		}

		if idx != nb_proofs - 1 {
			println!("");
		}
	}
}
