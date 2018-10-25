use clap;

use common::*;
use context;

/// Create the init subcommand.
pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	clap::SubCommand::with_name("init")
		.about("initialize a reserve file")
		.arg(
			clap::Arg::with_name("challenge")
				.long("challenge")
				.short("c")
				.help("the challenge string")
				.takes_value(true)
				.required(true),
		).arg(
			clap::Arg::with_name("block-number")
				.long("block-number")
				.short("b")
				.help("the block number the proofs are to be valid at")
				.takes_value(true)
				.required(false),
		)
}

/// Execute the init command.
pub fn execute(ctx: &mut context::Ctx) {
	let mut p = ProofFile::new(ctx.network());
	p.version = 1;

	match ctx.command().value_of("challenge") {
		None => panic!("challenge not provided"),
		Some("") => panic!("empty challenge is not allowed"),
		Some(challenge) => {
			p.challenge = String::from(challenge);
		}
	};

	if let Some(bn) = ctx.command().value_of("block-number") {
		p.block_number = bn.parse().expect("failed to parse block number");
	}

	debug!("Creating proof file: {:?}", &p);
	ctx.save_proof_file(p);
}
