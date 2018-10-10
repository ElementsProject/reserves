use bitcoin;
use protos;
pub use protos::{Network, Proof_Status};

//TODO(stevenroose) when supporting Liquid, we can make an enum Proof that will hold either
// a Bitcoin(bitcoin::Proof) or Liquid(liquid::Proof).

pub struct ProofFile {
	pub version: u32,
	pub network: Network,
	pub challenge: String,
	pub block_number: u32,
	pub proofs: Vec<bitcoin::Proof>,
}

impl From<protos::ProofOfReserves> for ProofFile {
	fn from(p: protos::ProofOfReserves) -> Self {
		ProofFile {
			version: p.version,
			network: p.network,
			challenge: p.challenge.into(),
			block_number: p.block_number,
			//TODO(stevenroose) proofs: p.take_proofs().into_vec().into_iter().into().collect(),
			proofs: p
				.proofs
				.into_iter()
				.map(|p| {
					let i: bitcoin::Proof = p.into();
					i
				}).collect(),
		}
	}
}

impl Into<protos::ProofOfReserves> for ProofFile {
	fn into(self) -> protos::ProofOfReserves {
		let mut p = protos::ProofOfReserves::new();
		p.set_version(self.version);
		p.set_network(self.network);
		p.set_challenge(self.challenge);
		p.set_block_number(self.block_number);
		//TODO(stevenroose) p.set_proofs(self.proofs.into_iter().into().collect());
		p.set_proofs(
			self.proofs
				.into_iter()
				.map(|p| {
					let i: protos::Proof = p.into();
					i
				}).collect(),
		);
		p
	}
}

impl ProofFile {
	pub fn new(network: Network) -> Self {
		ProofFile {
			version: 0,
			network: network,
			challenge: String::new(),
			block_number: 0,
			proofs: vec![],
		}
	}

	/// Find a proof with the given id.
	pub fn take_proof(&mut self, id: &str) -> Option<bitcoin::Proof> {
		let mut found = None;
		for (idx, proof) in self.proofs.iter().enumerate() {
			if proof.id == id {
				found = Some(idx);
			}
		}

		if let Some(idx) = found {
			Some(self.proofs.remove(idx))
		} else {
			None
		}
	}
}
