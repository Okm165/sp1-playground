use crate::utils::AffinePointV2;
use crate::{syscall_secp256k1_add, syscall_secp256k1_double};

/// The number of limbs in [Bn254AffinePoint].
pub const N: usize = 16;

/// An affine point on the Secp256k1 curve.
#[derive(Copy, Clone)]
pub struct Secp256k1AffinePoint(pub [u32; N]);

impl AffinePointV2<N> for Secp256k1AffinePoint {
    /// The values are taken from https://en.bitcoin.it/wiki/Secp256k1.
    const GENERATOR: [u32; N] = [
        385357720, 1509065051, 768485593, 43777243, 3464956679, 1436574357, 4191992748, 2042521214,
        4212184248, 2621952143, 2793755673, 4246189128, 235997352, 1571093500, 648266853,
        1211816567,
    ];

    fn new(limbs: [u32; N]) -> Self {
        Self(limbs)
    }

    fn limbs_ref(&self) -> &[u32; N] {
        &self.0
    }

    fn limbs_mut(&mut self) -> &mut [u32; N] {
        &mut self.0
    }

    fn add_assign(&mut self, other: &Self) {
        let a = self.limbs_mut();
        let b = other.limbs_ref();
        unsafe {
            syscall_secp256k1_add(a.as_mut_ptr(), b.as_ptr());
        }
    }

    fn double(&mut self) {
        let a = self.limbs_mut();
        unsafe {
            syscall_secp256k1_double(a.as_mut_ptr());
        }
    }
}
