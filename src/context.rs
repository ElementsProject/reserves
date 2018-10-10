use std::fs;

use clap;
use protobuf;
use protobuf::Message;

use common;
use protos;

pub struct Ctx<'a> {
	pub matches: &'a clap::ArgMatches<'a>,
}

impl<'a> Ctx<'a> {
	fn proof_file_path(&self) -> &str {
		self.matches.value_of("proof-file").expect("--proof-file cannot be empty")
	}

	pub fn load_proof_file(&self) -> common::ProofFile {
		let path = self.proof_file_path();
		let mut file = fs::File::open(path).expect(&format!("error opening file at '{}'", path));
		let pf: protos::ProofOfReserves =
			protobuf::parse_from_reader(&mut file).expect("error parsing reserve file");
		if pf.get_version() != 1 {
			panic!("Unknown proof file version: {}", pf.get_version())
		}
		pf.into()
	}

	pub fn save_proof_file(&self, pf: common::ProofFile) {
		let path = self.proof_file_path();
		let mut file = fs::File::create(path).expect(&format!("error opening file at '{}'", path));
		let proto: protos::ProofOfReserves = pf.into();
		proto.write_to_writer(&mut file).expect("error writing reserve file");
	}

	pub fn command(&self) -> &'a clap::ArgMatches<'a> {
		self.matches.subcommand().1.unwrap()
	}

	pub fn network(&self) -> protos::Network {
		//TODO(stevenroose) change with --liquid flag or --testnet flag
		protos::Network::BITCOIN
	}
}
