use std::fs;

use clap;
use protobuf;
use protobuf::Message;

use common;
use protos;
use utils;

pub fn global_args<'a>() -> Vec<clap::Arg<'a, 'a>> {
	vec![
		clap::Arg::with_name("verbose")
			.short("v")
			.multiple(true)
			.takes_value(false)
			.help("print verbose logging output to stderr")
			.global(true),
		clap::Arg::with_name("proof-file")
			.long("proof-file")
			.short("f")
			.help("the proof-of-reserves file to use")
			.takes_value(true)
			.default_value("reserves.proof")
			.global(true),
		clap::Arg::with_name("testnet")
			.long("testnet")
			.takes_value(false)
			.help("use the Bitcoin testnet network")
			.global(true),
		clap::Arg::with_name("dry-run")
			.short("n")
			.long("dry-run")
			.takes_value(false)
			.help("perform a dry run: no changes will be made to the proof file")
			.global(true),
	]
}

pub struct Ctx<'a> {
	pub matches: &'a clap::ArgMatches<'a>,
	network: Option<protos::Network>, // lazily determine
}

impl<'a> Ctx<'a> {
	pub fn new(matches: &'a clap::ArgMatches) -> Ctx<'a> {
		Ctx {
			matches: matches,
			network: None,
		}
	}

	fn proof_file_path(&self) -> &str {
		self.matches.value_of("proof-file").expect("--proof-file cannot be empty")
	}

	pub fn load_proof_file(&mut self) -> common::ProofFile {
		let mut file = {
			let path = self.proof_file_path();
			fs::File::open(path).expect(&format!("error opening file at '{}'", path))
		};
		let pf: protos::ProofOfReserves =
			protobuf::parse_from_reader(&mut file).expect("error parsing reserve file");
		if pf.get_version() != 1 {
			panic!("Unknown proof file version: {}", pf.get_version())
		}
		let proof_network = pf.get_network();
		if let Some(args_network) = self.args_network() {
			if args_network != proof_network {
				panic!(
					"Proof file network ({}) incompatible with network from CLI flag ({})",
					utils::network_name(proof_network),
					utils::network_name(args_network)
				);
			}
		}
		self.network = Some(proof_network);
		pf.into()
	}

	pub fn save_proof_file(&self, pf: common::ProofFile) {
		if self.dry_run() {
			println!("Dry-run: not writing proof file to disk.");
			return;
		}

		let path = self.proof_file_path();
		let mut file = fs::File::create(path).expect(&format!("error opening file at '{}'", path));
		let proto: protos::ProofOfReserves = pf.into();
		proto.write_to_writer(&mut file).expect("error writing reserve file");
	}

	pub fn command(&self) -> &'a clap::ArgMatches<'a> {
		self.matches.subcommand().1.unwrap()
	}

	pub fn verbosity(&self) -> usize {
		self.matches.occurrences_of("verbose") as usize
	}

	// The network explicitly specified by cli arguments.
	fn args_network(&self) -> Option<protos::Network> {
		if self.matches.is_present("testnet") {
			Some(protos::Network::BITCOIN_TESTNET)
		} else {
			None
		}
	}

	pub fn network(&self) -> protos::Network {
		match self.network {
			// use the one we found when loading the proof file
			Some(network) => network,
			// fallback to command line args
			None => match self.args_network() {
				Some(network) => network,
				// fallback to Bitcoin mainnet
				None => protos::Network::BITCOIN_MAINNET,
			},
		}
	}

	pub fn dry_run(&self) -> bool {
		self.matches.is_present("dry-run")
	}
}
