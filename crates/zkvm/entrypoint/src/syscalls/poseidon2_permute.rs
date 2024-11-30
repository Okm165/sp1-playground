#[cfg(target_os = "zkvm")]
use core::arch::asm;

/// Executes the Poseidon2 permutation on the given state.
///
/// ### Safety
///
/// The caller must ensure that `state` is valid pointer to data that is aligned along a four
/// byte boundary.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_poseidon2_permute(input: *const [u32; 16], output: *mut [u32; 16]) {
    #[cfg(target_os = "zkvm")]
    unsafe {
        asm!(
            "ecall",
            in("t0") crate::syscalls::POSEIDON2_PERMUTE,
            in("a0") input,
            in("a1") output
        );
    }

    #[cfg(not(target_os = "zkvm"))]
    unreachable!()
}
