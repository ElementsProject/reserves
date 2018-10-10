extern crate bitcoin as rbitcoin;
extern crate bitcoin_amount;
extern crate bitcoinconsensus;
extern crate bitcoindrpc;
extern crate clap;
extern crate crypto;
extern crate hex;
extern crate protobuf;

use std::panic;
use std::process;

use clap::{App, AppSettings, Arg};

mod backend;
mod bitcoin;
mod cmd;
mod common;
mod context;
mod protos;
mod utils;

fn main() {
	// Apply a custom panic hook to print a more user-friendly message
	// in case the execution fails.
	panic::set_hook(Box::new(|info| {
		let message = if let Some(m) = info.payload().downcast_ref::<String>() {
			m
		} else if let Some(m) = info.payload().downcast_ref::<&str>() {
			m
		} else {
			"No message provided"
		};
		println!("Execution failed: {}", message);
		process::exit(1);
	}));

	let matches = App::new("reserves")
		.version("0.0.0")
		.author("Steven Roose <steven@blockstream.io>")
		.about("Proof-of-Reserves generator and verifier")
		.setting(AppSettings::SubcommandRequiredElseHelp)
		.setting(AppSettings::AllArgsOverrideSelf)
		.arg(
			Arg::with_name("proof-file")
				.long("proof-file")
				.short("f")
				.help("the proof-of-reserves file to use")
				.takes_value(true)
				.default_value("reserves.proof"),
		)
		.subcommand(cmd::init::subcommand())
		.subcommand(cmd::inspect::subcommand())
		.subcommand(cmd::drop::subcommand())
		//.subcommand(cmd::add_proof::subcommand())
		.subcommand(cmd::verify::subcommand())
		.subcommand(cmd::add_utxos::subcommand())
		.subcommand(cmd::drop_utxos::subcommand())
		.subcommand(cmd::sign::subcommand())
		.get_matches();

	let mut ctx = context::Ctx {
		matches: &matches,
	};

	// Execute other commands.
	match matches.subcommand() {
		("init", _) => cmd::init::execute(&mut ctx),
		("inspect", _) => cmd::inspect::execute(&mut ctx),
		("drop", _) => cmd::drop::execute(&mut ctx),
		//("add-proof", Some(sub)) => cmd::add_proof::execute(&mut ctx, sub),
		("verify", _) => cmd::verify::execute(&mut ctx),
		("add-utxos", _) => cmd::add_utxos::execute(&mut ctx),
		("drop-utxos", _) => cmd::drop_utxos::execute(&mut ctx),
		("sign", _) => cmd::sign::execute(&mut ctx),
		(c, _) => println!("command {} unknown", c),
	};
}
