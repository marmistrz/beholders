use kzg_traits::{EcBackend, Fr, G1Mul, KZGSettings, G1};
pub(crate) type PublicKey<B: EcBackend> = B::G1;
pub(crate) type SecretKey<B: EcBackend> = B::Fr;

#[derive(Debug)]
pub(crate) struct Schnorr<B: EcBackend> {
    pub(crate) a: B::G1,
    pub(crate) c: B::Fr,
    pub(crate) z: B::Fr,
}

struct BareSchnorr<B: EcBackend> {
    c: B::Fr,
    z: B::Fr,
}

impl<B: EcBackend> Schnorr<B> {
    pub fn verify(&self, pk: &PublicKey<B>) -> bool {
        let g = B::G1::generator();
        pk.mul(&self.c).add(&self.a) == g.mul(&self.z)
    }

    pub fn prove(sk: &B::Fr, r: &B::Fr, c: B::Fr) -> Self {
        let g = B::G1::generator();
        let a = g.mul(&r);
        let z = r.add(&c.mul(&sk));
        Self { a, c, z }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kzg::{eip_7594::BlstBackend, types::fr::FsFr};
    type Backend = BlstBackend;

    #[test]
    fn test_schnorr() {
        let g = <Backend as EcBackend>::G1::generator();

        let r = FsFr::from_u64(1337);
        let sk = FsFr::from_u64(42);
        let pk = g.mul(&sk);
        let c = FsFr::from_u64(2137);
        let proof = Schnorr::<Backend>::prove(&sk, &r, c);
        assert!(proof.verify(&pk));
    }
}
