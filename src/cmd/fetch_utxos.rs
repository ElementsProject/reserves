use clap;

use backend;
use bitcoin;
use cmd::common;
use common::*;
use context;

/// Create the fetch-utxos subcommand.
pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	clap::SubCommand::with_name("fetch-utxos")
		.about("fetch UTXOs from a wallet to add to a new or existing proof")
		.arg(common::id_arg())
		.args(&backend::bitcoind::args())
}

pub fn fetch_utxos(command: &clap::ArgMatches) -> Vec<bitcoin::UTXO> {
	// currently only bitcoind
	if let Some(mut bitcoind) = backend::bitcoind::Backend::load(command) {
		bitcoind.fetch_utxos()
	} else {
		panic!("No argument provided with which we can fetch UTXOs!")
	}
}

/// Execute the fetch-utxos command.
pub fn execute(ctx: &mut context::Ctx) {
	let mut pf = ctx.load_proof_file();

	let proof_id = ctx.command().value_of("id").expect("no proof identifier given");

	let mut proof = pf
		.take_proof(proof_id)
		.or_else(|| Some(bitcoin::Proof::new(proof_id.to_owned(), Proof_Status::GATHERING_UTXOS)))
		.unwrap();

	let utxos = fetch_utxos(ctx.command());
	println!("Retrieved {} UTXOs from source", utxos.len());

	// Add the UTXOs to the proof.
	let len_before = proof.utxos.len();
	for utxo in utxos.into_iter() {
		// Add if not yet present.
		if proof.utxos.iter().find(|u| u.point == utxo.point).is_some() {
			continue;
		}

		debug!("Adding UTXO: {:?}", utxo);
		proof.utxos.push(utxo);
	}
	let added = proof.utxos.len() - len_before;
	println!("Added {} UTXOs to the proof", added);

	pf.proofs.insert(0, proof);
	ctx.save_proof_file(pf);
}
