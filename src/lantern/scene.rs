use nalgebra::Vector3;

pub struct Scene {
    pub spheres: Vec<Sphere>,
    pub materials: Vec<Material>,
}

pub struct Material {
    pub albedo: Vector3<f32>,
    pub roughness: f32,
    pub metallic: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            albedo: Vector3::new(1.0, 1.0, 1.0),
            roughness: 1.0,
            metallic: 0.0,
        }
    }
}

pub struct Sphere {
    pub position: Vector3<f32>,
    pub radius: f32,
    pub material_index: usize,
}

impl Default for Sphere {
    fn default() -> Self {
        Self {
            position: Vector3::zeros(),
            radius: 1.0,
            material_index: 0,
        }
    }
}
