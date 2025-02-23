use crate::types::{TFr, TG1};
use kzg_traits::{Fr, G1Mul, G1};

/// Secret key for Schnorr signature
pub(crate) type PublicKey = TG1;
/// Secret key for Schnorr signature
pub(crate) type SecretKey = TFr;

#[derive(Debug)]
pub(crate) struct Schnorr {
    pub(crate) a: TG1,
    pub(crate) c: TFr,
    pub(crate) z: TFr,
}

impl Schnorr {
    pub fn verify(&self, pk: &PublicKey) -> bool {
        let g = TG1::generator();
        pk.mul(&self.c).add(&self.a) == g.mul(&self.z)
    }

    pub fn prove(sk: &TFr, r: &TFr, c: TFr) -> Self {
        let g = TG1::generator();
        let a = g.mul(r);
        let z = r.add(&c.mul(sk));
        Self { a, c, z }
    }
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
        let c = FsFr::from_u64(2137);
        let proof = Schnorr::prove(&sk, &r, c);
        assert!(proof.verify(&pk));
    }
}
