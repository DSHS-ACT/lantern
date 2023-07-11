use std::ops::Add;
use nalgebra::{Matrix4, Perspective3, Point3, Rotation3, Unit, Vector2, Vector3, Vector4};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

pub struct Camera {
    projection: Perspective3<f32>,
    view: Matrix4<f32>,
    inverse_view: Matrix4<f32>,

    vertical_fov: f32,
    near: f32,
    far: f32,

    pub position: Point3<f32>,
    forward: Unit<Vector3<f32>>,

    pub rays: Vec<Vector3<f32>>,
    last_mouse: PhysicalPosition<f64>,

    viewport_size: PhysicalSize<u32>,

    inputs: [bool; 6] // WASD SPACE SHIFT
}

impl Camera {
    pub fn new(vertical_fov: f32, near: f32, far: f32, viewport_size: PhysicalSize<u32>) -> Self {
        let aspect = viewport_size.width as f32 / viewport_size.height as f32;
        let projection = Perspective3::new(aspect, vertical_fov, near, far);
        let position = Point3::origin();
        let forward = Vector3::z_axis();
        let view: Matrix4<f32> = Matrix4::look_at_lh(&position, &Point3::from(forward.data.0[0]), &Vector3::y_axis());

        let inverse_view = view.try_inverse().unwrap();

        Self {
            projection,
            view,
            inverse_view,
            vertical_fov,
            near,
            far,
            position,
            forward,
            rays: vec![],
            last_mouse: Default::default(),
            viewport_size,
            inputs: [false; 6],
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            /*WindowEvent::CursorMoved { position, .. } => {
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
                self.forward.renormalize_fast();

                self.reevaluate_view();
                self.reevaluate_rays();

                true
            }*/
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state,
                    virtual_keycode: Some(key),
                    ..
                },
                ..
            } => {
                let is_press = matches!(state, ElementState::Pressed);
                match key {
                    VirtualKeyCode::W => self.inputs[0] = is_press,
                    VirtualKeyCode::A => self.inputs[1] = is_press,
                    VirtualKeyCode::S => self.inputs[2] = is_press,
                    VirtualKeyCode::D => self.inputs[3] = is_press,
                    VirtualKeyCode::Space => self.inputs[4] = is_press,
                    VirtualKeyCode::LShift => self.inputs[5] = is_press,
                    _ => {
                        return false
                    }
                };

                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, frame_time: u128) {
        let time_step = ((frame_time as f32) / 1000.0).min(1.0 / 60.0);

        let up: Unit<Vector3<f32>> = Vector3::y_axis();
        let right = Unit::new_unchecked(self.forward.cross(&up));
        let mut moved = false;

        if self.inputs[0] {
            self.position += self.forward.scale(self.movement_speed() * time_step);
            moved = true;
        }
        if self.inputs[1] {
            self.position -= right.scale(self.movement_speed() * time_step);
            moved = true;
        }
        if self.inputs[2] {
            self.position -= self.forward.scale(self.movement_speed() * time_step);
            moved = true;
        }
        if self.inputs[3] {
            self.position += right.scale(self.movement_speed() * time_step);
            moved = true;
        }
        if self.inputs[4] {
            self.position += up.scale(self.movement_speed() * time_step);
            moved = true;
        }
        if self.inputs[5] {
            self.position -= up.scale(self.movement_speed() * time_step);
            moved = true;
        }

        if moved {
            self.reevaluate_view();
            self.reevaluate_rays();
        }
    }

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
        self.projection = Perspective3::new(aspect, self.vertical_fov, self.near, self.far);
    }

    fn reevaluate_view(&mut self) {
        let point = self.position.add(self.forward.into_inner());
        self.view = Matrix4::look_at_lh(&self.position, &point, &Vector3::y_axis());
        self.inverse_view = self.view.try_inverse().unwrap();
    }

    fn reevaluate_rays(&mut self) {
        let mut new_rays = Vec::with_capacity((self.viewport_size.width * self.viewport_size.height) as usize);
        let ratio = self.viewport_size.width as f32 / self.viewport_size.height as f32;
        dbg!(ratio);

        for y in 0..self.viewport_size.height {
            for x in 0..self.viewport_size.width {
                let mut coord = Vector2::new(
                    x as f32 / self.viewport_size.width as f32,
                    y as f32 / self.viewport_size.height as f32,
                );
                coord *= 2.0;
                coord -= Vector2::new(1.0, 1.0);

                let target = self.projection.as_matrix() * Vector4::new(coord.x, coord.y, 1.0,1.0);
                let normalized = (target.xyz() / target.w).normalize();

                let ray_direction = self.inverse_view * Vector4::new(normalized.x, normalized.y, normalized.z, 0.0);

                new_rays.push(ray_direction.xyz());
            }
        }

        self.rays = new_rays;
    }
}