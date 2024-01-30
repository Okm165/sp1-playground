#![no_main]

extern crate succinct_zkvm;

succinct_zkvm::entrypoint!(main);

extern "C" {
    fn syscall_secp_add(p: *mut u32, q: *const u32);
}

// TODO: Update the constants

pub fn main() {
    // generator.
    // 55066263022277343669578718895168534326250603453777594175500187360389116729240
    // 32670510020758816978083085130507043184471273380659243275938904335757337482424
    let mut a: [u8; 64] = [
        152, 23, 248, 22, 91, 129, 242, 89, 217, 40, 206, 45, 219, 252, 155, 2, 7, 11, 135, 206,
        149, 98, 160, 85, 172, 187, 220, 249, 126, 102, 190, 121, 184, 212, 16, 251, 143, 208, 71,
        156, 25, 84, 133, 166, 72, 180, 23, 253, 168, 8, 17, 14, 252, 251, 164, 93, 101, 196, 163,
        38, 119, 218, 58, 72,
    ];

    // 2 * generator.
    // 89565891926547004231252920425935692360644145829622209833684329913297188986597
    // 12158399299693830322967808612713398636155367887041628176798871954788371653930
    let b: [u8; 64] = [
        197, 189, 200, 77, 201, 212, 57, 105, 191, 133, 123, 170, 167, 50, 114, 38, 37, 102, 188,
        29, 215, 227, 157, 142, 252, 31, 129, 67, 24, 255, 114, 136, 115, 94, 94, 55, 43, 200, 117,
        224, 139, 251, 238, 45, 80, 154, 70, 213, 219, 78, 201, 108, 73, 203, 72, 45, 167, 131,
        199, 47, 82, 134, 53, 62,
    ];

    unsafe {
        syscall_secp_add(a.as_mut_ptr() as *mut u32, b.as_ptr() as *const u32);
    }

    // 3 * generator.
    // 112711660439710606056748659173929673102114977341539408544630613555209775888121
    // 25583027980570883691656905877401976406448868254816295069919888960541586679410
    let c: [u8; 64] = [
        249, 54, 224, 188, 19, 241, 1, 134, 176, 153, 111, 131, 69, 200, 49, 181, 41, 82, 157, 248,
        133, 79, 52, 73, 16, 195, 88, 146, 1, 138, 48, 249, 114, 230, 184, 132, 117, 253, 185, 108,
        27, 35, 194, 52, 153, 169, 0, 101, 86, 243, 55, 42, 230, 55, 227, 15, 20, 232, 45, 99, 15,
        123, 143, 56,
    ];

    assert_eq!(a, c);

    println!("done");
}
