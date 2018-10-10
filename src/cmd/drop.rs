use clap;

use cmd::common;
use context::*;

/// Create the drop subcommand.
pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	clap::SubCommand::with_name("drop").about("drop a proof").arg(common::id_arg())
}

/// Execute the drop command.
pub fn execute(ctx: &mut Ctx) {
	let mut pf = ctx.load_proof_file();

	let proof_id = ctx.command().value_of("id").expect("no proof identifier given");

	let nb_proofs = pf.proofs.len();
	pf.proofs.retain(|p| p.id != proof_id);

	if nb_proofs <= pf.proofs.len() {
		println!("No proofs with id '{}' found.", proof_id);
	} else if nb_proofs - pf.proofs.len() == 1 {
		println!("Proof successfully dropped.");
	} else {
		println!(
			"{} proofs with id '{}' were successfully dropped.",
			nb_proofs - pf.proofs.len(),
			proof_id
		);
	}
}
