//! Backend-specific types
use kzg::{eip_7594::BlstBackend, types::fk20_single_settings::FsFK20SingleSettings};
use kzg_traits::EcBackend;

type Backend = BlstBackend;
pub type TG1 = <Backend as EcBackend>::G1;
pub type TG2 = <Backend as EcBackend>::G2;
pub type TFr = <Backend as EcBackend>::Fr;
pub type TKZGSettings = <Backend as EcBackend>::KZGSettings;
pub type TPoly = <Backend as EcBackend>::Poly;
pub type TFFTSettings = <Backend as EcBackend>::FFTSettings;
pub type TFK20SingleSettings = FsFK20SingleSettings;
