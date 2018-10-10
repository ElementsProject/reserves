use std::collections::HashSet;
use std::str::FromStr;

use clap;
use rbitcoin::blockdata::transaction::OutPoint;

use cmd::common;
use common::*;
use context;

/// Create the drop-utxos subcommand.
pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	clap::SubCommand::with_name("drop-utxos")
		.about("drop some UTXOs from a proof")
		.arg(common::id_arg())
		.arg(
			clap::Arg::with_name("utxo")
				.multiple(true)
				.takes_value(true)
				.help("UTXOs to drop in the format of <txid>:<vout>"),
		)
}

/// Execute the drop-utxos command.
pub fn execute(ctx: &mut context::Ctx) {
	let mut pf = ctx.load_proof_file();

	let proof_id = ctx.command().value_of("id").expect("no proof identifier given");

	let mut proof = pf.take_proof(proof_id).expect("No proof found with given id");

	match proof.status {
		Proof_Status::UNDEFINED => panic!("Corrupt proof file"),
		Proof_Status::SIGNING => panic!("Proof is already in SIGNING state"),
		Proof_Status::FINAL => panic!("Proof already in FINAL state"),
		Proof_Status::GATHERING_UTXOS => { /* ok */ }
	}

	let mut drops = HashSet::new();
	for utxo in ctx.command().values_of("utxo").expect("no UTXOs provided") {
		drops.insert(OutPoint::from_str(utxo).expect(&format!("failed to parse UTXO: {}", utxo)));
	}

	let nb_before = proof.utxos.len();
	proof.utxos.retain(|u| !drops.contains(&u.point));

	println!("Dropped {} UTXOs.", nb_before - proof.utxos.len());

	pf.proofs.insert(0, proof);
	ctx.save_proof_file(pf);
}
