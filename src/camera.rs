use std::ops::Add;
use nalgebra::{Isometry3, IsometryMatrix3, Matrix4, Perspective3, Point3, Quaternion, Rotation3, Unit, UnitQuaternion, Vector2, Vector3, Vector4};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

pub struct Camera {
    projection: Perspective3<f32>,
    view: Isometry3<f32>,

    vertical_fov: f32,
    near: f32,
    far: f32,

    pub position: Point3<f32>,
    forward: Unit<Vector3<f32>>,

    pub rays: Vec<Vector3<f32>>,
    pub last_mouse: PhysicalPosition<f64>,

    viewport_size: PhysicalSize<u32>,

    inputs: [bool; 6], // WASD SPACE SHIFT
    pub grab_mouse: bool
}

impl Camera {
    pub fn new(vertical_fov: f32, near: f32, far: f32, viewport_size: PhysicalSize<u32>) -> Self {
        let aspect = viewport_size.width as f32 / viewport_size.height as f32;
        let projection = Perspective3::new(aspect, vertical_fov, near, far);
        let position = Point3::origin();
        let forward = Unit::new_unchecked(Vector3::new(0.0, 0.0, 1.0));
        let view = Isometry3::look_at_lh(&position, &Point3::from(forward.data.0[0]), &Vector3::y_axis());

        Self {
            projection,
            view,
            vertical_fov,
            near,
            far,
            position,
            forward,
            rays: vec![],
            last_mouse: Default::default(),
            viewport_size,
            inputs: [false; 6],
            grab_mouse: false
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

                let q = UnitQuaternion::from_axis_angle(&right, pitch_delta)
                    * UnitQuaternion::from_axis_angle(&up, yaw_delta);

                self.forward = q * self.forward;
                self.forward.renormalize_fast();

                self.reevaluate_view();
                self.reevaluate_rays();

                true
            }
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
                    VirtualKeyCode::C if is_press => {
                        self.grab_mouse = !self.grab_mouse;
                    }
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
        let right = self.forward.cross(&up).normalize();
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
            dbg!(self.position);
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
        0.7
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
        self.view = Isometry3::face_towards(&self.position, &point, &Vector3::new(0.0, 1.0, 0.0));
    }

    fn reevaluate_rays(&mut self) {
        let mut new_rays = Vec::with_capacity((self.viewport_size.width * self.viewport_size.height) as usize);

        for y in 0..self.viewport_size.height {
            for x in 0..self.viewport_size.width {
                let mut coord = Vector2::new(
                    x as f32 / self.viewport_size.width as f32,
                    y as f32 / self.viewport_size.height as f32,
                );
                coord *= 2.0;
                coord -= Vector2::new(1.0, 1.0);

                let target = self.projection.as_matrix() * Vector4::new(coord.x, coord.y, 1.0,1.0);
                let normalized = (target.xyz()).normalize();

                let ray_direction = self.view.inverse_transform_vector(&normalized);

                new_rays.push(ray_direction);
            }
        }

        self.rays = new_rays;
    }
}