use clap;
use hex;
use rbitcoin::consensus::encode::deserialize;
use rbitcoin::Transaction;

use bitcoin;
use cmd::common;
use common::*;
use context;

/// Create the add-proof subcommand.
pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	clap::SubCommand::with_name("add-proof")
		.about("add a raw proof transaction")
		.arg(common::id_arg())
		.arg(
			clap::Arg::with_name("proof-tx")
				.help("the hexadecimal proof tx")
				.takes_value(true)
				.required(true),
		)
}

/// Execute the add-proof command.
pub fn execute(ctx: &mut context::Ctx) {
	let mut pf = ctx.load_proof_file();

	let proof_id = ctx.command().value_of("id").expect("no proof identifier given");

	if pf.take_proof(proof_id).is_some() {
		panic!("A proof with this ID already exists.");
	}

	let hex_tx = ctx.command().value_of("proof-tx").expect("no proof tx provided");
	let raw_tx = hex::decode(hex_tx).expect("proof tx not hex");
	let tx: Transaction = deserialize(&raw_tx).expect("invalid transaction encoding");

	// Perform some validation of the tx.
	if tx.input.len() < 2 {
		panic!("Proof transaction has less than two inputs.");
	}
	if tx.input[0] != bitcoin::challenge_txin(&pf.challenge) {
		panic!("Proof transaction does not commit to the correct challenge.");
	}

	let mut proof = bitcoin::Proof::new(proof_id.to_owned(), Proof_Status::FINAL);
	proof.proof_tx = Some(tx);

	pf.proofs.insert(0, proof);
	ctx.save_proof_file(pf);
}
