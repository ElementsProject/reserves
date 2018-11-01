use std::collections::HashMap;

use clap;
use hex;
use rbitcoin::consensus::encode::deserialize;
use rbitcoin::util::bip32;
use rbitcoin::util::psbt;
use secp256k1;

use bitcoin;
use cmd::common;
use common::*;
use context;

/// Create the add-utxo subcommand.
pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	clap::SubCommand::with_name("add-utxo")
		.about("manually add a UTXO to a proof")
		.arg(common::id_arg())
		.args(&vec![
			// Necessary argument.
			clap::Arg::with_name("outpoint")
				.help("the outpoint of the UTXO (`<txid>:<vout>` format)")
				.takes_value(true)
				.required(true),
			// PSBT arguments.
			clap::Arg::with_name("previous-tx")
				.long("previous-tx")
				.help("the encoded transaction the UTXO is part of")
				.takes_value(true),
			clap::Arg::with_name("previous-output")
				.long("previous-output")
				.help("the encoded transaction output of the UTXO")
				.takes_value(true),
			clap::Arg::with_name("hd-keypath")
				.long("hd-keypath")
				.help("the BIP32 HD wallet keypath this UTXO has to be signed with")
				.takes_value(true),
			clap::Arg::with_name("redeem-script")
				.long("redeem-script")
				.help("the redeem script needed to spend the UTXO")
				.takes_value(true),
			clap::Arg::with_name("witness-script")
				.long("witness-script")
				.help("the witness script needed to spend the UTXO")
				.takes_value(true),
			//TODO(stevenroose) add HD keypaths
			// Metadata arguments.
			clap::Arg::with_name("block-number")
				.long("block-number")
				.help("the number of the block the UTXO was created in")
				.takes_value(true),
			clap::Arg::with_name("block-hash")
				.long("block-hash")
				.help("the hash of the block the UTXO was created in")
				.takes_value(true),
		])
}

/// Execute the add-utxo command.
pub fn execute(ctx: &mut context::Ctx) {
	let mut pf = ctx.load_proof_file();
	let proof_id = ctx.command().value_of("id").expect("no proof identifier given");

	let outpoint_str = ctx.command().value_of("outpoint").expect("outpoint is required");
	let outpoint = outpoint_str.parse().expect("failed to parse outpoint");

	let mut proof = pf
		.take_proof(proof_id)
		.or_else(|| Some(bitcoin::Proof::new(proof_id.to_owned(), Proof_Status::GATHERING_UTXOS)))
		.unwrap();
	if let Some(ref utxo) = proof.utxos.iter().find(|u| u.point == outpoint) {
		info!("UTXO found with given outpoint: {:?}", utxo);
		panic!("Proof already has a UTXO with this outpoint.");
	}

	let utxo = bitcoin::UTXO {
		point: outpoint,
		psbt_input: psbt::Input {
			non_witness_utxo: match ctx.command().value_of("previous-tx") {
				Some(t) => Some(
					deserialize(&hex::decode(t).expect("invalid previous tx hex"))
						.expect("invalid previous tx encoding"),
				),
				None => None,
			},
			witness_utxo: match ctx.command().value_of("previous-output") {
				Some(t) => Some(
					deserialize(&hex::decode(t).expect("invalid previous output hex"))
						.expect("invalid previous output encoding"),
				),
				None => None,
			},
			hd_keypaths: match ctx.command().value_of("hd-keypath") {
				Some(p) => {
					//TODO(stevenroose) what to do with public key and fingerprint?
					// Trezor doesn't need those
					let mut map = HashMap::new();
					let path =
						bip32::parse_derivation_path(&p).expect("failed to parse HD keypath");
					let secp = secp256k1::Secp256k1::signing_only();
					let mut bytes = vec![0; secp256k1::constants::SECRET_KEY_SIZE];
					bytes[0] = 1;
					let empty_privkey =
						secp256k1::key::SecretKey::from_slice(&secp, &bytes).unwrap();
					let pubkey = secp256k1::key::PublicKey::from_secret_key(&secp, &empty_privkey);
					map.insert(pubkey, (Default::default(), path));
					map
				}
				None => HashMap::new(),
			},
			redeem_script: match ctx.command().value_of("redeem-script") {
				Some(t) => Some(
					deserialize(&hex::decode(t).expect("invalid redeem script hex"))
						.expect("invalid redeem script encoding"),
				),
				None => None,
			},
			witness_script: match ctx.command().value_of("witness-script") {
				Some(t) => Some(
					deserialize(&hex::decode(t).expect("invalid witness script hex"))
						.expect("invalid witness script encoding"),
				),
				None => None,
			},
			..Default::default()
		},
		block_number: match ctx.command().value_of("block-number") {
			Some(n) => n.parse().expect("failed to parse block number"),
			None => 0,
		},
		block_hash: match ctx.command().value_of("block-hash") {
			Some(h) => Some(h.parse().expect("failed to parse block hash")),
			None => None,
		},
	};

	debug!("Adding new UTXO to proof: {:?}", utxo);
	proof.utxos.push(utxo);
	println!("Successfully added the UTXO to the proof.");

	pf.proofs.insert(0, proof);
	ctx.save_proof_file(pf);
}
