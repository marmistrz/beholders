use crate::types::{TFr, TG1};
use kzg_traits::{Fr, G1Mul, G1};
use serde::{Deserialize, Serialize};

/// Secret key for Schnorr signature
pub type PublicKey = TG1;
/// Secret key for Schnorr signature
pub type SecretKey = TFr;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct Schnorr {
    pub(crate) a: TG1,
    pub(crate) c: u32,
    pub(crate) z: TFr,
}

impl Schnorr {
    pub fn verify(&self, pk: &PublicKey) -> bool {
        let g = TG1::generator();
        let c = TFr::from_u64(self.c.into());
        pk.mul(&c).add(&self.a) == g.mul(&self.z)
    }

    pub fn prove(sk: &TFr, r: &TFr, c: u32) -> Self {
        let cfr = TFr::from_u64(c.into());

        let g = TG1::generator();
        let a = g.mul(r);
        let z = r.add(&cfr.mul(sk));
        Self { a, c, z }
    }
}

pub fn maxc(difficulty: u32) -> u32 {
    1u32 << difficulty
}

#[cfg(test)]
mod tests {
    use super::*;
    use kzg::types::fr::FsFr;

    #[test]
    fn test_schnorr() {
        let g = TG1::generator();

        let r = FsFr::from_u64(1337);
        let sk = FsFr::from_u64(42);
        let pk = g.mul(&sk);
        let c = 2137;
        let proof = Schnorr::prove(&sk, &r, c);
        assert!(proof.verify(&pk));
    }
}
