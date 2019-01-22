use std::io::Write;

use clap;
use rbitcoin::blockdata::script::Script;
use rbitcoin::blockdata::transaction::Transaction;
use rbitcoin::consensus::encode::deserialize as bitcoin_deserialize;
use rbitcoin::network::constants::Network as BitcoinNetwork;
use rbitcoin::util::psbt;
use rpassword;
use trezor::{self, SignTxProgress, Trezor, TrezorMessage, TrezorResponse};

use bitcoin;
use context::Ctx;

pub fn args<'a>() -> Vec<clap::Arg<'a, 'a>> {
	vec![
		clap::Arg::with_name("trezor")
			.long("trezor")
			.help("use a Trezor hardware wallet to sign")
			.takes_value(false),
	]
}

pub struct Backend(Trezor);

/// Handle user interactions with Trezor when asked for device PIN or passphrase.
fn handle_interaction<T, R: TrezorMessage>(resp: TrezorResponse<T, R>) -> T {
	match resp {
		TrezorResponse::Ok(res) => res,
		TrezorResponse::Failure(_) => resp.ok().expect("received failure from Trezor device"),
		TrezorResponse::ButtonRequest(req) => {
			println!("Please follow the instructions shown on the Trezor screen...");
			handle_interaction(req.ack().expect("Trezor error"))
		}
		TrezorResponse::PinMatrixRequest(req) => {
			let pin = rpassword::prompt_password_stdout("Enter PIN: ").unwrap();
			handle_interaction(req.ack_pin(pin).expect("Trezor error"))
		}
		TrezorResponse::PassphraseRequest(req) => {
			if req.on_device() {
				println!("Please provide your passphrase on the Trezor device.");
				handle_interaction(req.ack().expect("Trezor error"))
			} else {
				let pass = rpassword::prompt_password_stdout("Enter passphrase: ").unwrap();
				handle_interaction(req.ack_passphrase(pass).expect("Trezor error"))
			}
		}
		TrezorResponse::PassphraseStateRequest(req) => {
			debug!("Passphrase state received: {:?}", req.passphrase_state());
			handle_interaction(req.ack().expect("Trezor error"))
		}
	}
}

fn tx_progress(
	psbt: &mut psbt::PartiallySignedTransaction,
	progress: SignTxProgress,
	network: BitcoinNetwork,
	signed_tx_buf: &mut Vec<u8>,
) {
	// If the device provided a part of the serialized signed tx, write it to the buffer.
	if let Some(signed_tx_part) = progress.get_serialized_tx_part() {
		signed_tx_buf.write(signed_tx_part).unwrap();
	}

	if !progress.finished() {
		// We need to do some special magic to make the Trezor ignore the challenge input.
		let is_challenge_input = {
			let req = progress.tx_request();
			req.has_request_type()
				&& req.has_details()
				&& req.get_request_type() == trezor::protos::TxRequest_RequestType::TXINPUT
				&& !req.get_details().has_tx_hash()
				&& req.get_details().has_request_index()
				&& req.get_details().get_request_index() == 0
		};
		let progress = if is_challenge_input {
			// For the challenge input, we provide the TxAck message manually to Trezor because
			// we want to fill in some non-standard values.
			let input = &psbt.global.unsigned_tx.input[0];
			let mut data_input = trezor::protos::TxAck_TransactionType_TxInputType::new();
			data_input
				.set_prev_hash(trezor::utils::to_rev_bytes(&input.previous_output.txid).to_vec());
			data_input.set_prev_index(input.previous_output.vout);
			data_input.set_script_sig(input.script_sig.to_bytes());
			data_input.set_sequence(input.sequence);
			data_input.set_amount(0);
			// This is the most important part. By setting the script type to SPENDWITNESS,
			// Trezor will assume that the value of the input will be confirmed when signing.
			data_input.set_script_type(trezor::protos::InputScriptType::SPENDWITNESS);
			//TODO(stevenroose) replace this with EXTERNAL once it works:
			// https://github.com/trezor/trezor-core/issues/388

			let mut txdata = trezor::protos::TxAck_TransactionType::new();
			txdata.mut_inputs().push(data_input);
			let mut msg = trezor::protos::TxAck::new();
			msg.set_tx(txdata);
			handle_interaction(progress.ack_msg(msg).expect("Trezor error"))
		} else {
			handle_interaction(progress.ack_psbt(&psbt, network).expect("Trezor error"))
		};
		tx_progress(psbt, progress, network, signed_tx_buf)
	}
}

impl Backend {
	pub fn load<'a>(matches: &'a clap::ArgMatches) -> Option<Self> {
		if !matches.is_present("trezor") {
			return None;
		}

		let mut trezor = trezor::unique(Some(false)).expect("error discovering Trezor device");
		trezor.init_device().expect("error initializing Trezor device");
		Some(Backend(trezor))
	}

	/// Ask Trezor to sign the given tx.
	pub fn sign_tx(
		&mut self,
		ctx: &Ctx,
		psbt: &mut psbt::PartiallySignedTransaction,
	) -> Transaction {
		let btc_network = bitcoin::network(ctx.network());

		// Initiate the signing with Trezor.
		let resp = self.0.sign_tx(psbt, btc_network).expect("Trezor error signing tx");

		// Work through the signing flow and accumulate changes to the psbt and the signed tx.
		let mut signed_tx = Vec::new();
		tx_progress(psbt, handle_interaction(resp), btc_network, &mut signed_tx);

		// Parse the signed tx received from Trezor.
		let mut signed: Transaction =
			bitcoin_deserialize(&signed_tx).expect("created invalid signed tx with Trezor");

		// Because the challenge input is not supported by Trezor, we tricked it into thinking it
		// was a normal one.  Because of that, it signed the input.  We have to remove the signature
		// because it's not valid there.
		signed.input[0].script_sig = Script::new();
		signed
	}
}
