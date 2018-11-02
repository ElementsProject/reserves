use std::collections::HashSet;

use bitcoin_amount::Amount;
use clap;

use backend;
use common::*;
use context;
use protos;

/// Create the verify subcommand.
pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	clap::SubCommand::with_name("verify")
		.about("verify the proofs in the proof file")
		.args(&backend::bitcoind::args())
}

/// Execute the verify command.
pub fn execute(ctx: &mut context::Ctx) {
	let pf = ctx.load_proof_file();

	// Check if any UTXO is spent by multiple proofs.
	let mut outpoints = HashSet::new();
	for proof in pf.proofs.iter() {
		if proof.status != protos::Proof_Status::FINAL {
			panic!("Proof '{}' is not final yet.", proof.id);
		}

		for out in proof.spending_utxos().into_iter() {
			if !outpoints.insert(out) {
				panic!("UTXO {} is spent in two different proofs!", out);
			}
		}
	}
	println!("Total number of UTXOs: {}", outpoints.len());

	// Then verify all the proof txs.
	let mut total_amount = Amount::from_sat(0);
	for mut proof in pf.proofs.iter() {
		match pf.network {
			Network::BITCOIN_MAINNET | Network::BITCOIN_TESTNET => {
				let mut bitcoind = backend::bitcoind::Backend::load(ctx.command())
					.expect("failed to load bitcoind");
				let txouts = bitcoind.fetch_proof_prevouts(&proof, pf.block_number);
				let amount = proof.verify(&pf.challenge, txouts);
				total_amount = total_amount + amount;
				println!("Verified proof '{}' for {} satoshis.", proof.id, amount.into_inner());
			}
			Network::LIQUID => panic!("can't verify Liquid proofs yet"),
		}
	}

	println!("All proofs verified for the following challenge: \"{}\"", pf.challenge);
	println!("Total amount of reserves: {} satoshis", total_amount.into_inner());
}
