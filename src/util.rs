use std::{fs::File, io::BufReader, ops::BitXor, path::Path};

use anyhow::Context;
use kzg_traits::FFTSettings;
use serde::{de::DeserializeOwned, Serialize};

use crate::types::TFFTSettings;

/// Bitwise XOR of two arrays.
pub(crate) fn bitxor<T: BitXor, const N: usize>(
    x: [T; N],
    y: [T; N],
) -> [<T as BitXor>::Output; N] {
    let vec = x.into_iter().zip(y).map(|(a, b)| a ^ b).collect::<Vec<_>>();
    match vec.try_into() {
        Ok(arr) => arr,
        Err(_) => unreachable!(),
    }
}

#[macro_export]
macro_rules! check {
    ($expr:expr, $msg:expr $(,)?) => {
        if !$expr {
            eprintln!("{}", $msg);
            return Ok(false);
        }
    };
}

pub fn write_to_file<T: Serialize>(path: &std::path::Path, data: &T) -> anyhow::Result<()> {
    let file = std::fs::File::create(path).context(format!("Unable to create file: {:?}", path))?;
    let mut writer = std::io::BufWriter::new(file);
    bincode::serde::encode_into_std_write(data, &mut writer, bincode::config::standard())
        .context("Bincode serialization error")?;
    Ok(())
}

pub fn read_from_file<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let file = File::open(path).context(format!("Unable to open file: {:?}", path))?;
    let mut reader = BufReader::new(file);
    bincode::serde::decode_from_std_read(&mut reader, bincode::config::standard())
        .context("Bincode deserialization error")
}

/// Returns the FFT settings for a given data length.
pub fn fft_settings(chunks: usize) -> Result<TFFTSettings, String> {
    assert!(
        chunks.is_power_of_two(),
        "The number of chunks needs to be a power of two"
    );

    let scale = chunks.ilog2() + 1;
    TFFTSettings::new(scale.try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitxor() {
        let x = [1, 2, 3];
        let y = [4, 5, 6];
        let z = bitxor(x, y);
        assert_eq!(z, [5, 7, 5]);
    }

    #[test]
    fn test_check() {
        fn a() -> Result<bool, ()> {
            check!(true, "");
            Err(())
        }

        fn b() -> Result<bool, ()> {
            check!(false, "");
            Err(())
        }

        assert!(a().is_err());
        assert_eq!(b(), Ok(false));
    }
}
