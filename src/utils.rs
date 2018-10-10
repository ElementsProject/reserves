use crypto::digest::Digest;
use crypto::sha2::Sha256;

use protos;

/// Implements SHA-256.
pub fn sha256(data: &[u8]) -> [u8; 32] {
	let mut dig = Sha256::new();
	dig.input(data);
	let mut h = [0; 32];
	dig.result(&mut h);
	h
}

/// Human readable name for the networks.
pub fn network_name(network: protos::Network) -> String {
	match network {
		protos::Network::BITCOIN => "BITCOIN",
		protos::Network::LIQUID => "LIQUID",
	}.into()
}
