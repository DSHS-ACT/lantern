use std::iter;
use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use eframe::egui::{ClippedPrimitive, ComboBox, DragValue, FontData, FontDefinitions, FontFamily, Label, Widget};
use nalgebra::Vector3;
use wgpu::{Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder, CommandEncoderDescriptor, CompositeAlphaMode, Device, DeviceDescriptor, Dx12Compiler, Face, Features, FragmentState, FrontFace, include_wgsl, IndexFormat, Instance, InstanceDescriptor, Limits, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode, PowerPreference, PresentMode, PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, SamplerBindingType, ShaderStages, Surface, SurfaceConfiguration, SurfaceError, TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension, vertex_attr_array, VertexAttribute, VertexBufferLayout, VertexState};
use wgpu::BindingResource::{Sampler, TextureView};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event::WindowEvent::CursorMoved;
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::camera::Camera;
use crate::lantern::Lantern;
use crate::lantern::scene::{Material, Scene, Sphere};

pub struct Application {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    main_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    blit_bind_group: BindGroup,
    // 무조건 winit의 Window를 쓸 것!
    pub window: Window,
    show_egui: bool,
    egui_state: egui_winit::State,
    egui_context: eframe::egui::Context,
    egui_renderer: egui_wgpu::Renderer,
    egui_screen: egui_wgpu::renderer::ScreenDescriptor,
    pub lantern: Lantern,
    camera: Camera,
    scene: Scene,
}

impl Application {
    // Rust식 생성자. new라는 이름의 메서드를 만듦
    pub async fn new(window: Window, event_loop: &EventLoop<()>) -> Self {
        let size = window.inner_size();

        // instance는 Adapter와 Surface를 만들어주며 이들에 필요한 정보를 제공함.
        // 백엔드: Vulkan, Metal, DirectX 등등
        // 아래 메서드를 호출하면 아무 백엔드나 상관 없는 instnace를 요청함
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),                // 모든 종류의 백엔드 허용
            dx12_shader_compiler: Dx12Compiler::default(), // DirectX 사용시, 쉐이더 컴파일러로 FXC 사용
        });

        // unsafe: 메모리 오류 등을 발생시킬 수도 있는 메서드
        // 아래는 create_surface가 안전하지 않아서 unsafe로 감쌈
        // 전달하는 &window가 생성하는 surface보다 오래 유지되어야 함.
        // 그리고 여기서 surface는 GPU가 그릴 수 있는 사각형 "표면"을 의미함.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        // instance라는 Graphic API 관리자로 Adapter 가져오기
        // adapter는 GPU 관리자. 기기로부터 정보를 가져오거나 특정 요청을 보낼 수 있음.
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(), // 전원 설정. 저전력과 고성능 모드가 있음. 기본값으로 저전력 모드.
                force_fallback_adapter: false, // 만약 사용 가능한 어뎁더가 없으면 CPU에서 렌더링함. 느림. 그래서 안쓸꺼임.
                // 계다가 따로 adapter나 instance에서 제한 건거도 없는데 아무것도 안될리가...
                compatible_surface: Some(&surface), // 어뎁더를 요청할 때, 요청하는 어뎁더가 무조건 특정 Surface와 호환되야 함을 강제함.
            })
            .await
            .unwrap();
        // await: 준비될 때 까지 기다리기
        // unwrap: request_adapter는 제시된 조건에 맞는 어뎁더를 찾지 못할 수 있으니 어뎁더를 Option에 감싸는데,
        // 이가 존재하지 않으면 바로 튕기게 함.

        // device: GPU 장치
        // queue: GPU에 보낼 명령어들을 저장하는 큐
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    features: Features::empty(), // 사용할 기능 없음
                    // 기능들: https://docs.rs/wgpu/latest/wgpu/struct.Features.html
                    // limits는 버퍼 사이즈, 텍스쳐 크기와 같은 제한 사항 기준들
                    limits: if cfg!(target_arch = "wasm32") {
                        // 브라우저가 아직 webgpu를 제대로 지원 안하니 모든 브라우저에서 지원할만한 webgl2 기준 채택
                        Limits::downlevel_webgl2_defaults()
                    } else {
                        Limits::default()
                    },
                    // 디버그시 device에 붙일 이름
                    label: Some("Lantern GPU"),
                },
                None, // 무슨 Graphic API를 호출하는지 기록할 경로. 우린 그런거 안할꺼임.
            )
            .await
            .unwrap();

        let camera = Camera::new(45.0, 0.1, 100.0, size);
        let scene = Scene {
            spheres: vec![
                Sphere {
                    position: Vector3::new(0.0, -101.0, 0.0),
                    radius: 100.0,
                    material_index: 0,
                },
                Sphere {
                    position: Vector3::zeros(),
                    radius: 0.5,
                    material_index: 1,
                },
            ],
            materials: vec![
                Material {
                    albedo: Vector3::new(0.2, 0.3, 1.0),
                    ..Material::default()
                },
                Material {
                    albedo: Vector3::new(1.0, 0.0, 1.0),
                    ..Material::default()
                },
            ],
        };

        let lantern = Lantern::new(&device, size);

        // 해당 surface랑 adapter가 가진 기능들의 집합
        let capabilities = surface.get_capabilities(&adapter);

        // 색 포맷으로 sRGB 사용, 이는 RGB 값을 어떤 물리적인 색상으로 변환할 것인가를 정의함.
        // sRGB 말고 다른거 쓰면 의도한 것보다 밝기나 명도에서 차이가 날 수 있음.
        let surface_format = capabilities
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(capabilities.formats[0]);
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT, // 해당 surface를 이용해 화면에 작성할 것임.
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync, // 렌더링된 결과물이랑 모니터에 물리적으로 표시된거랑 어떻게 동기화 할지 결정
            alpha_mode: CompositeAlphaMode::Auto, // 알파값을 이용한 투명도 연산 방법 지정. 자동으로 설정하게 함.
            view_formats: vec![], // 아래 get_current_texture 호출 시 사용할 수도 있는 대체 텍스쳐 포맷들.
            // 그런거 없으니 빈 벡터 사용.
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(include_wgsl!("./shader.wgsl"));

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: BufferUsages::INDEX,
        });

        let blit_bind_group_layout = device.create_bind_group_layout(&BLIT_BIND_GROUP_LAYOUT);
        let blit_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout: &blit_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: TextureView(&lantern.final_image.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: Sampler(&lantern.final_image.sampler),
                }
            ],
        });

        let main_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Main Pipeline Layout"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });
        let main_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Main Pipeline"),
            layout: Some(&main_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::layout()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let egui_state = egui_winit::State::new(event_loop);
        let egui_context = eframe::egui::Context::default();

        // 한글 지원 추가
        let fonts = {
            // eframe의 Font Definitions
            let mut default = FontDefinitions::default();

            // eframe의 FontData
            default.font_data.insert(
                String::from("Nanum Gothic"),
                FontData::from_static(include_bytes!("../NanumGothic.ttf")),
            );

            // eframe::egui::FontFamily
            default.families.insert(FontFamily::Proportional, vec![String::from("Nanum Gothic")]);

            default
        };
        egui_context.set_fonts(fonts);

        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            surface_format,
            None, // 깊이 안씀
            1, // 멀티 샘플링 1번만 할꺼임
        );
        let egui_screen = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: [config.width, config.height],
            pixels_per_point: egui_context.pixels_per_point(),
        };

        Self {
            surface,
            device,
            queue,
            config,
            size,
            main_pipeline,
            vertex_buffer,
            index_buffer,
            blit_bind_group,
            window,
            show_egui: false,
            egui_state,
            egui_context,
            egui_renderer,
            egui_screen,
            lantern,
            camera,
            scene,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);

        self.egui_screen.pixels_per_point = self.egui_context.pixels_per_point();
        self.egui_screen.size_in_pixels = [self.config.width, self.config.height];
        self.lantern.resize(&self.device, new_size);
        self.camera.resize(new_size);

        self.blit_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout: &self.device.create_bind_group_layout(&BLIT_BIND_GROUP_LAYOUT),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: TextureView(&self.lantern.final_image.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: Sampler(&self.lantern.final_image.sampler),
                }
            ],
        });
    }

    pub fn update(&mut self, frame_time: u128) {
        if self.camera.update(frame_time) {
            self.lantern.reset_counter();
        }
        self.lantern.update(&self.scene, &self.camera, &self.queue);

        if self.camera.grab_mouse {
            let center = PhysicalPosition::new(self.size.width / 2, self.size.height / 2);
            if let Err(e) = self.window.set_cursor_position(center) {
                eprintln!("{e}");
            }

            self.camera.last_mouse.x = center.x as f64;
            self.camera.last_mouse.y = center.y as f64;
        }
    }

    pub fn render(&mut self, frame_time: u128) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?; // 렌더링 결과를 출력할 곳

        // view는 위에서 가져온 output을 다뤄주는 것임.
        let view = output.texture.create_view(&TextureViewDescriptor::default());
        // encoder는 GPU에 보내는 명령들을 임시적으로 저장하는 것
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Encoder"),
        });

        // render_pass가 encoder를 빌려오기 때문에 아래처럼 따로 빼지 않으면 앞으로 계속 쓸 수 없음
        {
            let primitives = if self.show_egui {
                self.update_egui(&mut encoder, frame_time)
            } else {
                vec![]
            };
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                // RenderpassColorAttachment: 해당 Render Pass에 가져다 붙일 색상을 지정함
                // color_attachments를 Option으로 전달하는 이유는
                // 특정 파이프라인은 아래 배열에 요소가 여러개 있어야만 하는데
                // 필요 없으면 그냥 None 전달할 수 있도록 하기 위해서
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view, // 렌더링할 결과를 저장할 때 사용할 view
                    // 멀티샘플링 사용시 텍스쳐의 최종 결과를 저장할 텍스쳐 View
                    // 우린 그런거 없으니 &view를 사용함.
                    // 근데 None 전달하면 얘가 자동으로 &view를 사용해줌.
                    resolve_target: None,
                    // ops는 이전 프레임 색상을 가지고 무엇을 할지 결정해줌
                    ops: Operations {
                        // load는 색상을 어디서 불러올건지 지정.
                        // Clear랑 Load가 있는데, Load는 이전 프레임 색상 가져오기, Clear는 그냥 단색 쓰기
                        load: LoadOp::Clear(Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        // 처리한 색상을 위에서 지정한 view에 작성할지 말지 지정
                        // 우린 단색으로 도배하니 언제나 true로 설정
                        store: true,
                    },
                })],
                // 깊이맵, 스텐실은 아직 안쓰니 None
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.main_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(0..));
            render_pass.set_index_buffer(self.index_buffer.slice(0..), IndexFormat::Uint16);
            render_pass.set_bind_group(0, &self.blit_bind_group, &[]);
            render_pass.draw_indexed(0..6, 0, 0..1);

            if self.show_egui {
                self.egui_renderer.render(&mut render_pass, &primitives, &self.egui_screen)
            }
        }

        // 위에서 render_pass를 이용해 작성한 내용을 이제는 담고 있을 encoder를 마감하고 queue를 통해 device에 전송
        self.queue.submit(iter::once(encoder.finish()));
        // 전송 끝났으면 모니터에 출력
        output.present();

        // 프레임 생성 성공!
        Ok(())
    }

    // true: 앱에서 입력 처리를 했으니 따로 관리할 필요 없음
    // false: 아래 event loop에서 처리 해야 함.
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        if self.egui_state.on_event(&self.egui_context, event).consumed {
            return true;
        }

        if let WindowEvent::MouseInput {
            state: ElementState::Pressed, button: MouseButton::Right, ..
        } = event {
            self.show_egui = !self.show_egui;
            return true;
        };
        let is_hovering = self.egui_context.is_pointer_over_area();

        let has_camera_consumed = self.camera.input(event, is_hovering);

        if has_camera_consumed {
            if let CursorMoved { .. } = event {
                self.lantern.reset_counter();
            };
        };

        has_camera_consumed
    }

    fn update_egui(&mut self, encoder: &mut CommandEncoder, frame_time: u128) -> Vec<ClippedPrimitive> {
        let egui_input = self.egui_state.take_egui_input(&self.window);
        let egui_output = self.egui_context.run(egui_input, |ctx| {
            eframe::egui::Window::new("설정")
                .resizable(true)
                .show(ctx, |ui| {
                    Label::new(format!("프레임 처리 시간: {} ms", (frame_time)))
                        .wrap(false)
                        .ui(ui);
                    ui.checkbox(&mut self.lantern.settings.should_accumulate, "Accumulate?");

                    if ui.button("Reset").clicked() {
                        self.lantern.reset_counter();
                    }

                    // 이름 붙이기 귀찮으니 일단 인덱스를 이름처럼 쓰기
                    ui.separator();
                    self.scene.spheres.iter_mut().enumerate().for_each(|(idx, sphere)| {
                        ui.collapsing(format!("구체 {idx}"), |ui| {
                            ui.horizontal(|ui| {
                                ui.label("위치:");
                                DragValue::new(&mut sphere.position.x).ui(ui);
                                DragValue::new(&mut sphere.position.y).ui(ui);
                                DragValue::new(&mut sphere.position.z).ui(ui);
                            });
                            ui.horizontal(|ui| {
                                ui.label("반지름:");
                                DragValue::new(&mut sphere.radius).ui(ui);
                            });
                            ComboBox::from_label("Material")
                                .selected_text(format!("Material {}", sphere.material_index))
                                .wrap(false)
                                .show_ui(ui, |ui| {
                                    for i in 0..self.scene.materials.len() {
                                        ui.selectable_value(&mut sphere.material_index, i, format!("Material {i}"));
                                    }
                                })
                        });
                    });

                    ui.separator();
                    self.scene.materials.iter_mut().enumerate().for_each(|(idx, material)| {
                        ui.collapsing(format!("Material {idx}"), |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Metallic:");
                                // 양수로 범위 제한
                                DragValue::new(&mut material.metallic)
                                    .max_decimals(4)
                                    .clamp_range(0.0..=1.0)
                                    .speed(0.05)
                                    .ui(ui);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Roughness:");
                                // 양수로 범위 제한
                                DragValue::new(&mut material.roughness)
                                    .speed(0.05)
                                    .clamp_range(0.0..=1.0)
                                    .ui(ui);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Albedo:");
                                ui.color_edit_button_rgb(&mut material.albedo.data.0[0]);
                            });
                        });
                    });
                });
        });

        self.egui_state.handle_platform_output(&self.window, &self.egui_context, egui_output.platform_output);
        let primitives = self.egui_context.tessellate(egui_output.shapes);
        egui_output.textures_delta.set.iter().for_each(|(id, delta)| {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, delta);
        });

        self.egui_renderer.update_buffers(&self.device, &self.queue, encoder, &primitives, &self.egui_screen);

        primitives
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [VertexAttribute; 2] = vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    pub fn layout<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

const BLIT_BIND_GROUP_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: Some("Blit Bind Group Layout"),
    entries: &[
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        }
    ],
};

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [1.0, 1.0, 0.0],
        tex_coord: [1.0, 1.0],
    },
    Vertex {
        position: [-1.0, 1.0, 0.0],
        tex_coord: [0.0, 1.0],
    },
    Vertex {
        position: [-1.0, -1.0, 0.0],
        tex_coord: [0.0, 0.0],
    },
    Vertex {
        position: [1.0, -1.0, 0.0],
        tex_coord: [1.0, 0.0],
    },
];

const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];
