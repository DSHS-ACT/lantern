use nalgebra::Vector3;

pub struct Scene {
    pub spheres: Vec<Sphere>
}

pub struct Sphere {
    pub position: Vector3<f32>,
    pub radius: f32,
    pub albedo: Vector3<f32>
}

impl Default for Sphere {
    fn default() -> Self {
        Self {
            position: Vector3::zeros(),
            radius: 1.0,
            albedo: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}