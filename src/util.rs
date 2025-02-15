use std::ops::BitXor;

use sha2::digest::typenum::Bit;

pub(crate) fn bitxor<T: BitXor, const N: usize>(
    x: [T; N],
    y: [T; N],
) -> [<T as BitXor>::Output; N] {
    let vec = x
        .into_iter()
        .zip(y.into_iter())
        .map(|(a, b)| a ^ b)
        .collect::<Vec<_>>();
    match vec.try_into() {
        Ok(arr) => arr,
        Err(_) => unreachable!(),
    }
}

#[macro_export]
macro_rules! check {
    ($expr:expr $(,)?) => {
        if !$expr {
            return Ok(false);
        }
    };
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
            check!(true);
            Err(())
        }

        fn b() -> Result<bool, ()> {
            check!(false);
            Err(())
        }

        assert!(a().is_err());
        assert_eq!(b(), Ok(false));
    }
}
