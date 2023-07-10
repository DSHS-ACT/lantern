use std::ops::Add;
use nalgebra::{Matrix4, Perspective3, Point3, Rotation3, Unit, Vector2, Vector3, Vector4};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

pub struct Camera {
    projection: Matrix4<f32>,
    view: Matrix4<f32>,
    inverse_projection: Matrix4<f32>,
    inverse_view: Matrix4<f32>,

    vertical_fov: f32,
    near: f32,
    far: f32,

    position: Point3<f32>,
    forward: Unit<Vector3<f32>>,

    rays: Vec<Vector3<f32>>,
    last_mouse: PhysicalPosition<f64>,

    viewport_size: PhysicalSize<u32>,
}

impl Camera {
    pub fn new(vertical_fov: f32, near: f32, far: f32, viewport_size: PhysicalSize<u32>) -> Self {
        let aspect = viewport_size.width as f32 / viewport_size.height as f32;
        let projection = Perspective3::new(aspect, vertical_fov, near, far).to_homogeneous();
        let position = Point3::origin();
        let forward = Vector3::z_axis();
        let view: Matrix4<f32> = Matrix4::look_at_lh(&position, &Point3::from(forward.data.0[0]), &Vector3::y_axis());

        let inverse_projection = projection.try_inverse().unwrap();
        let inverse_view = view.try_inverse().unwrap();

        Self {
            projection,
            view,
            inverse_projection,
            inverse_view,
            vertical_fov,
            near,
            far,
            position,
            forward,
            rays: vec![],
            last_mouse: Default::default(),
            viewport_size,
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let delta = Vector2::new(
                    (position.x - self.last_mouse.x) as f32,
                    (position.y - self.last_mouse.y) as f32,
                ) * 0.002;
                self.last_mouse = *position;

                let up: Unit<Vector3<f32>> = Vector3::y_axis();
                let right = Unit::new_unchecked(self.forward.cross(&up));

                let pitch_delta = delta.y * self.rotation_speed();
                let yaw_delta = delta.x * self.rotation_speed();

                let pitch_rotation = Rotation3::from_axis_angle(&right, -pitch_delta);
                let yaw_rotation = Rotation3::from_axis_angle(&up, -yaw_delta);

                let combined = pitch_rotation * yaw_rotation;

                self.forward = combined * self.forward;

                assert!((0.98..1.02).contains(&self.forward.magnitude_squared()));
                self.reevaluate_view();
                self.reevaluate_rays();

                true
            }
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: Some(key),
                    ..
                },
                ..
            } => {
                // todo
                match key {
                    VirtualKeyCode::W => {
                        //self.position += self.forward * self.movement_speed() *
                    }
                    _ => {}
                };
                false
            }
            _ => false,
        }
    }

    pub fn update(&self, time_step: f32) {}

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.viewport_size = new_size;

        self.reevaluate_projection();
        self.reevaluate_rays();
    }

    pub fn rotation_speed(&self) -> f32 {
        0.3
    }

    pub fn movement_speed(&self) -> f32 {
        5.0
    }

    fn reevaluate_projection(&mut self) {
        let aspect = self.viewport_size.width as f32 / self.viewport_size.height as f32;
        self.projection = Perspective3::new(aspect, self.vertical_fov, self.near, self.far).to_homogeneous();
        self.inverse_projection = self.projection.try_inverse().unwrap();
    }

    fn reevaluate_view(&mut self) {
        let point = self.position.add(self.forward.into_inner());
        self.view = Matrix4::look_at_lh(&self.position, &point, &Vector3::y_axis());
        self.inverse_view = self.view.try_inverse().unwrap();
    }

    fn reevaluate_rays(&mut self) {
        let mut new_rays = Vec::with_capacity((self.viewport_size.width * self.viewport_size.height) as usize);

        for y in 0..self.viewport_size.height {
            for x in 0..self.viewport_size.width {
                let coord = Vector2::new(
                    x as f32 / self.viewport_size.width as f32,
                    y as f32 / self.viewport_size.height as f32,
                ) * 2.0 - Vector2::new(1.0, 1.0);

                let mut target = self.inverse_projection * Vector4::new(coord.x, coord.y, 1.0, 1.0); // 월드 좌표
                target /= target.w;

                let mut normalized: Vector4<f32> = target.clone();
                normalized.w = 0.0;
                normalized = normalized.normalize();

                let ray_direction = (self.inverse_view * normalized).xyz();

                new_rays.push(ray_direction);
            }
        }

        self.rays = new_rays;
    }
}