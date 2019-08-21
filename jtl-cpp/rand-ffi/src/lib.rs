use rand_chacha::ChaChaRng;
use rand_core::{SeedableRng, RngCore};
use rand::Rng;

#[derive(Clone)]
pub struct Random(ChaChaRng);

#[no_mangle]
pub unsafe extern "C" fn random_init(seed: [u8; 32]) -> *mut Random {
    let chacha = ChaChaRng::from_seed(seed);
    Box::into_raw(Box::new(Random(chacha)))
}

#[no_mangle]
pub unsafe extern "C" fn random_free(rnd: *mut Random) {
    Box::from_raw(rnd);
}

#[no_mangle]
pub unsafe extern "C" fn random_clone(rnd: &mut Random) -> *mut Random {
    let rnd2 = rnd.clone();
    Box::into_raw(Box::new(rnd2))
}

#[no_mangle]
pub unsafe extern "C" fn random_gen64(rnd: &mut Random) -> u64 {
    let out = rnd.0.next_u64();
    out
}

#[no_mangle]
pub unsafe extern "C" fn random_gen_range(rnd: &mut Random, lo: i64, hi: i64) -> i64 {
    let out = rnd.0.gen_range(lo, hi);
    out
}
