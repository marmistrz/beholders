//! Backend-specific types
use kzg::{eip_7594::BlstBackend, types::fk20_single_settings::FsFK20SingleSettings};
use kzg_traits::EcBackend;

type Backend = BlstBackend;
pub(crate) type TG1 = <Backend as EcBackend>::G1;
pub(crate) type TG2 = <Backend as EcBackend>::G2;
pub(crate) type TFr = <Backend as EcBackend>::Fr;
pub(crate) type TKZGSettings = <Backend as EcBackend>::KZGSettings;
pub(crate) type TPoly = <Backend as EcBackend>::Poly;
pub(crate) type TFFTSettings = <Backend as EcBackend>::FFTSettings;
pub(crate) type TFK20SingleSettings = FsFK20SingleSettings;
