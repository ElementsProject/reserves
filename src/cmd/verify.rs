use std::collections::HashSet;

use clap;

use backend;
use common::*;
use context;

/// Create the verify subcommand.
pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	clap::SubCommand::with_name("verify")
		.about("verify the proofs in the proof file")
		.args(backend::bitcoind::args().as_slice())
}

/// Execute the verify command.
pub fn execute(ctx: &mut context::Ctx) {
	let pf = ctx.load_proof_file();

	// Check if any UTXO is spent by multiple proofs.
	let mut outpoints = HashSet::new();
	for proof in pf.proofs.iter() {
		for out in proof.spending_utxos().into_iter() {
			if !outpoints.insert(out) {
				panic!("UTXO {} is spent in two different proofs!", out);
			}
		}
	}

	// Then verify all the proof txs.
	for mut proof in pf.proofs.iter() {
		match pf.network {
			Network::BITCOIN => {
				let mut bitcoind = backend::bitcoind::Backend::load(ctx.command())
					.expect("failed to load bitcoind");
				let txouts = bitcoind.fetch_proof_prevouts(&proof, pf.block_number);
				proof.verify(&pf.challenge, txouts);
			}
			Network::LIQUID => panic!("can't verify Liquid proofs yet"),
		}
	}

	println!("Proof verified for the following challenge: \"{}\"", pf.challenge);
}
