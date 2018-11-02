use clap;

use backend;
use bitcoin;
use cmd::common;
use common::*;
use context;

//TODO(stevenroose) Indicate which inputs should be signed; some signers don't really know.
// Alternative would be to afterwards verify which inputs are validly signed and keep those,
// discard others.

/// Create the sign subcommand.
pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	clap::SubCommand::with_name("sign")
		.about("sign a proof")
		.arg(common::id_arg())
		.args(&backend::bitcoind::args())
		.args(&backend::trezor::args())
}

/// Sign the tx with the active backend in the context.
pub fn sign_proof(ctx: &mut context::Ctx, proof: &mut bitcoin::Proof) {
	if let Some(mut trezor) = backend::trezor::Backend::load(ctx.command()) {
		let signed = trezor.sign_tx(ctx, &mut proof.psbt.as_mut().unwrap());
		//TODO(stevenroose) update psbt?
		proof.proof_tx = Some(signed);
		proof.status = Proof_Status::FINAL;
	} else if let Some(mut bitcoind) = backend::bitcoind::Backend::load(ctx.command()) {
		let signed = bitcoind.sign_tx(proof.psbt.clone().unwrap().global.unsigned_tx);
		//TODO(stevenroose) update psbt?
		proof.proof_tx = Some(signed);
		proof.status = Proof_Status::FINAL;
	} else {
		panic!("No argument provided with which we can sign txs!")
	}
}

/// Execute the sign command.
pub fn execute(ctx: &mut context::Ctx) {
	let mut pf = ctx.load_proof_file();

	let proof_id = ctx.command().value_of("id").expect("no proof identifier given");

	let mut proof = pf.take_proof(proof_id).expect("No proof found with given id");
	match proof.status {
		Proof_Status::SIGNING => { /* ok */ }
		Proof_Status::UNDEFINED => panic!("Corrupt proof file"),
		Proof_Status::FINAL => panic!("Proof already in final state"),
		Proof_Status::GATHERING_UTXOS => {
			// Done with outputs, set state to signing.
			proof.start_signing(&pf.challenge)
		}
	}

	sign_proof(ctx, &mut proof);
	println!("Successfully signed proof.");

	pf.proofs.insert(0, proof);
	ctx.save_proof_file(pf);
}
