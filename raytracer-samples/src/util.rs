use raytracer_impl::types::V3;

//
// Utility functions
//

pub fn deg_to_rad(deg: f32) -> f32 {
    (deg / 180.0) * std::f32::consts::PI
}

//
// Easing functions
//

pub fn lerp_v3(p1: V3, p2: V3, d: f32) -> V3 {
    let v_between = (p2 - p1) * d;
    p1 + v_between
}

pub fn lerp_f32(p1: f32, p2: f32, d: f32) -> f32 {
    let v_between = (p2 - p1) * d;
    p1 + v_between
}

pub fn ease_in(t: f32, scale: f32) -> f32 {
    // y = x ^ 2
    t.powf(scale)
}

pub fn ease_out(t: f32, scale: f32) -> f32 {
    // y = 1 - ((1 - x) ^ 2)
    1.0 - (1.0 - t).powf(scale)
}

pub fn ease_in_out(t: f32, scale: f32) -> f32 {
    lerp_f32(ease_in(t, scale), ease_out(t, scale), t)
}

//
// Random helpers
//

use rand_xorshift::XorShiftRng;

pub fn create_rng_from_seed(a: u128) -> XorShiftRng {
    use rand::SeedableRng;
    
    fn set_bytes(bytes: &mut [u8], val: u128) {
        for offset in 0..16 {
            let shift = (15 - offset) * 8;
            bytes[offset] = ((val >> shift) & 0xff) as u8
        }
    }

    let mut seed = <XorShiftRng as SeedableRng>::Seed::default();

    set_bytes(&mut seed[0..16], a);
    
    XorShiftRng::from_seed(seed)
}