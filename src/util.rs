use std::ops::RangeBounds;
use nalgebra::{Vector3};
use rand::{Rng, thread_rng};
use rand::distributions::uniform::{SampleRange, SampleUniform};

pub fn random_vec<T: SampleUniform, R: RangeBounds<T> + SampleRange<T> + Clone>(range: R) -> Vector3<T> {
    let mut rng = thread_rng();
    Vector3::new(
        rng.gen_range(range.clone()),
        rng.gen_range(range.clone()),
        rng.gen_range(range.clone())
    )
}