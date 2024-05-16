#![no_main]
sp1_zkvm::entrypoint!(main);

extern "C" {
    fn syscall_secp384r1_double(p: *mut u32);
}

pub fn main() {
    for _ in 0..10i64.pow(3) {
        // generator.
        // 26247035095799689268623156744566981891852923491109213387815615900925518854738050089022388053975719786650872476732087
        // 8325710961489029985546751289520108179287853048861315594709205902480503199884419224438643760392947333078086511627871
        let mut a: [u8; 96] = [
            183, 10, 118, 114, 56, 94, 84, 58, 108, 41, 85, 191, 93, 242, 2, 85, 56, 42, 84, 130,
            224, 65, 247, 89, 152, 155, 167, 139, 98, 59, 29, 110, 116, 173, 32, 243, 30, 199, 177,
            142, 55, 5, 139, 190, 34, 202, 135, 170, 95, 14, 234, 144, 124, 29, 67, 122, 157, 129,
            126, 29, 206, 177, 96, 10, 192, 184, 240, 181, 19, 49, 218, 233, 124, 20, 154, 40, 189,
            29, 244, 248, 41, 220, 146, 146, 191, 152, 158, 93, 111, 44, 38, 150, 74, 222, 23, 54,
        ];

        unsafe {
            syscall_secp384r1_double(a.as_mut_ptr() as *mut u32);
        }

        // 2 * generator.
        // 1362138308511466522361153706999924933599454966107597910086607881313301390679204654798639248640660900363360053616481
        // 21933325650940841369538204578070064804451893403314136885642470114978241170633179043576249504748352841115137159204480
        let b: [u8; 96] = [
            97, 223, 149, 82, 199, 169, 150, 91, 248, 100, 14, 190, 110, 232, 224, 79, 158, 110,
            185, 159, 209, 7, 210, 81, 214, 52, 244, 166, 89, 89, 2, 137, 240, 151, 91, 197, 69, 0,
            38, 105, 217, 210, 163, 123, 5, 153, 217, 8, 128, 14, 148, 10, 112, 30, 80, 97, 45,
            226, 57, 77, 233, 67, 253, 95, 37, 180, 106, 37, 95, 80, 78, 144, 62, 196, 108, 188,
            117, 216, 117, 178, 116, 186, 109, 253, 223, 232, 191, 183, 237, 60, 27, 91, 250, 241,
            128, 142,
        ];

        assert_eq!(a, b);
    }

    println!("done");
}
