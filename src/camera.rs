use std::mem::MaybeUninit;
use std::ops::Add;

use nalgebra::{
    Isometry3, Matrix4, Perspective3, Point3, Unit, UnitQuaternion, Vector2, Vector3, Vector4,
};
use rayon::prelude::*;
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

    pub rays: Vec<Unit<Vector3<f32>>>,
    pub last_mouse: PhysicalPosition<f64>,

    viewport_size: PhysicalSize<u32>,

    inputs: [bool; 6],
    // WASD SPACE SHIFT
    pub grab_mouse: bool,
}

impl Camera {
    pub fn new(vertical_fov: f32, near: f32, far: f32, viewport_size: PhysicalSize<u32>) -> Self {
        let aspect = viewport_size.width as f32 / viewport_size.height as f32;

        let projection = {
            let right = Perspective3::new(aspect, vertical_fov, near, far).into_inner();
            let mut z_flip = Matrix4::identity();
            z_flip[(2, 2)] = -1.0;
            Perspective3::from_matrix_unchecked(right * z_flip)
        };
        let position = Point3::from([0.0, 0.0, -1.0]);
        let forward = Vector3::z_axis();
        let target = position.add(&forward.into_inner());
        let rays = vec![];
        let view = Isometry3::look_at_lh(&position, &target, &Vector3::y_axis());

        let mut to_return = Self {
            projection,
            view,
            vertical_fov,
            near,
            far,
            position,
            forward,
            rays,
            last_mouse: Default::default(),
            viewport_size,
            inputs: [false; 6],
            grab_mouse: false,
        };

        to_return.reevaluate_rays();

        to_return
    }

    pub fn input(&mut self, event: &WindowEvent, is_hovering: bool) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } if !is_hovering => {
                let delta = Vector2::new(
                    (position.x - self.last_mouse.x) as f32,
                    (position.y - self.last_mouse.y) as f32,
                ) * 0.002;
                self.last_mouse = *position;

                let up: Unit<Vector3<f32>> = Vector3::y_axis();
                let right = Unit::new_unchecked(up.cross(&self.forward));

                let pitch_delta = delta.y * self.rotation_speed(); // negative when up
                let yaw_delta = delta.x * self.rotation_speed(); // positive when right

                let q = UnitQuaternion::from_axis_angle(&right, pitch_delta)
                    * UnitQuaternion::from_axis_angle(&up, yaw_delta);

                self.forward = q * self.forward;
                self.forward.renormalize_fast();

                self.reevaluate_view();
                self.reevaluate_rays();

                true
            }
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
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
                        return false;
                    }
                };

                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, frame_time: u128) -> bool {
        let time_step = ((frame_time as f32) / 1000.0).min(1.0 / 60.0);

        let up: Unit<Vector3<f32>> = Vector3::y_axis();
        let right = up.cross(&self.forward);
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
        };

        moved
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

        let right = Perspective3::new(aspect, self.vertical_fov, self.near, self.far).into_inner();
        let mut z_flip = Matrix4::identity();
        z_flip[(2, 2)] = -1.0;
        self.projection = Perspective3::from_matrix_unchecked(right * z_flip);
    }

    fn reevaluate_view(&mut self) {
        let point = self.position.add(self.forward.clone_owned());
        self.view = Isometry3::look_at_lh(&self.position, &point, &Vector3::y_axis());
    }

    fn reevaluate_rays(&mut self) {
        self.rays =
            Vec::with_capacity((self.viewport_size.width * self.viewport_size.height) as usize);
        let writer = self.rays.spare_capacity_mut();

        writer
            .par_iter_mut()
            .enumerate()
            .for_each(|(index, ray_direction)| {
                let y = index as u32 / self.viewport_size.width;
                let x = index as u32 % self.viewport_size.width;

                let mut coord = Vector2::new(
                    x as f32 / self.viewport_size.width as f32,
                    y as f32 / self.viewport_size.height as f32,
                );
                coord *= 2.0;
                coord -= Vector2::new(1.0, 1.0);

                let target = self.projection.inverse() * Vector4::new(coord.x, coord.y, 1.0, 1.0);
                // Frustum is right handed, z is inverted

                //let normalized = (target.xyz() / target.w).normalize();
                let mut normalized = target.xyz().normalize();

                if target.w.is_sign_negative() {
                    normalized = -normalized;
                }

                let new_direction =
                    Unit::new_unchecked(self.view.inverse_transform_vector(&normalized));

                assert!(
                    0.9 <= new_direction.magnitude_squared()
                        && new_direction.magnitude_squared() <= 1.1
                );
                *ray_direction = MaybeUninit::new(Unit::new_unchecked(
                    self.view.inverse_transform_vector(&normalized),
                ));
            });

        unsafe {
            self.rays
                .set_len((self.viewport_size.width * self.viewport_size.height) as usize);
        }
    }
}
