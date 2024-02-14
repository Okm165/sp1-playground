#![no_main]
sp1_zkvm::entrypoint!(main);

extern "C" {
    fn syscall_ed_add(p: *mut u32, q: *const u32);
}

pub fn main() {
    // 90393249858788985237231628593243673548167146579814268721945474994541877372611
    // 33321104029277118100578831462130550309254424135206412570121538923759338004303
    let mut a: [u8; 64] = [
        195, 166, 157, 207, 218, 220, 175, 197, 111, 177, 123, 23, 73, 72, 114, 103, 28, 246, 66,
        207, 66, 146, 187, 234, 136, 238, 133, 145, 47, 196, 216, 199, 79, 31, 224, 30, 179, 122,
        51, 84, 116, 12, 4, 189, 198, 198, 190, 22, 71, 201, 143, 249, 92, 56, 147, 133, 92, 187,
        130, 33, 152, 19, 171, 73,
    ];

    // 61717728572175158701898635111983295176935961585742968051419350619945173564869
    // 28137966556353620208933066709998005335145594788896528644015312259959272398451
    let b: [u8; 64] = [
        197, 189, 200, 77, 201, 212, 57, 105, 191, 133, 123, 170, 167, 50, 114, 38, 37, 102, 188,
        29, 215, 227, 157, 142, 252, 31, 129, 67, 24, 255, 114, 136, 115, 94, 94, 55, 43, 200, 117,
        224, 139, 251, 238, 45, 80, 154, 70, 213, 219, 78, 201, 108, 73, 203, 72, 45, 167, 131,
        199, 47, 82, 134, 53, 62,
    ];

    unsafe {
        syscall_ed_add(a.as_mut_ptr() as *mut u32, b.as_ptr() as *const u32);
    }

    // 36213413123116753589144482590359479011148956763279542162278577842046663495729
    // 17093345531692682197799066694073110060588941459686871373458223451938707761683
    let c: [u8; 64] = [
        49, 144, 129, 197, 86, 163, 62, 48, 222, 208, 213, 200, 219, 90, 163, 54, 211, 248, 178,
        224, 238, 167, 235, 219, 251, 247, 189, 239, 194, 16, 16, 80, 19, 106, 20, 198, 72, 56,
        103, 111, 68, 201, 29, 107, 75, 208, 193, 232, 181, 186, 175, 22, 213, 187, 253, 125, 44,
        80, 222, 209, 159, 125, 202, 37,
    ];

    assert_eq!(a, c);
    println!("done");
}
