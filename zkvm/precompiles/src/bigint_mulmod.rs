use crate::syscall_uint256_mul;

const BIGINT_WIDTH_WORDS: usize = 8;

#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn sys_bigint(
    result: *mut [u32; BIGINT_WIDTH_WORDS],
    op: u32,
    x: *const [u32; BIGINT_WIDTH_WORDS],
    y: *const [u32; BIGINT_WIDTH_WORDS],
    modulus: *const [u32; BIGINT_WIDTH_WORDS + 1],
) {
    // Instantiate a new uninitialized array of words to place the concatenated y and modulus.
    // Note that the modulus gets passed in as BIGINT_WIDTH_WORDS + 1 words for the case where
    // the modulus = 1 << 256. This case is identified by the modulus argument being 0 everywhere.
    let mut concat_y_modulus =
        core::mem::MaybeUninit::<[u32; BIGINT_WIDTH_WORDS * 2 + 1]>::uninit();
    unsafe {
        let result_ptr = result as *mut u32;
        let x_ptr = x as *const u32;
        let y_ptr = y as *const u32;
        let concat_ptr = concat_y_modulus.as_mut_ptr() as *mut u32;

        // First copy the y value into the concatenated array.
        core::ptr::copy(y_ptr, concat_ptr, BIGINT_WIDTH_WORDS);

        // Then, copy the modulus value into the concatenated array. Add the width of the y value
        // to the pointer to place the modulus value after the y value.
        core::ptr::copy(
            modulus as *const u32,
            concat_ptr.add(BIGINT_WIDTH_WORDS),
            BIGINT_WIDTH_WORDS + 1,
        );

        // Copy x into the result array, as our syscall will write the result into the first input.
        core::ptr::copy(x as *const u32, result_ptr, BIGINT_WIDTH_WORDS);

        // Call the uint256_mul syscall to multiply the x value with the concatenated y and modulus.
        // This syscall writes the result in-place, so it will mutate the result ptr appropriately.
        syscall_uint256_mul(result_ptr, concat_ptr);
    }
}
