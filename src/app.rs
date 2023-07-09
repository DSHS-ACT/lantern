use std::iter;

use eframe::egui::{ClippedPrimitive, FontData, FontDefinitions, FontFamily, TextEdit};
use wgpu::{Backends, Color, CommandEncoder, CommandEncoderDescriptor, CompositeAlphaMode, Device, DeviceDescriptor, Dx12Compiler, Features, Instance, InstanceDescriptor, Limits, LoadOp, Operations, PowerPreference, PresentMode, Queue, RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, Surface, SurfaceConfiguration, SurfaceError, TextureUsages, TextureViewDescriptor};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

pub struct Application {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    // 무조건 winit의 Window를 쓸 것!
    pub window: Window,
    egui_state: egui_winit::State,
    egui_context: eframe::egui::Context,
    egui_renderer: egui_wgpu::Renderer,
    egui_screen: egui_wgpu::renderer::ScreenDescriptor
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

        let egui_state = egui_winit::State::new(event_loop);
        let egui_context = eframe::egui::Context::default();

        // 한글 지원 추가
        let fonts = {
            // eframe의 Font Definitions
            let mut default = FontDefinitions::default();

            // eframe의 FontData
            default.font_data.insert(
                String::from("Nanum Gothic"),
                FontData::from_static(include_bytes!("../NanumGothic.ttf"))
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
            1 // 멀티 샘플링 1번만 할꺼임
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
            window,
            egui_state,
            egui_context,
            egui_renderer,
            egui_screen,
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
    }

    pub fn update(&mut self) {}

    pub fn render(&mut self) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?; // 렌더링 결과를 출력할 곳

        // view는 위에서 가져온 output을 다뤄주는 것임.
        let view = output.texture.create_view(&TextureViewDescriptor::default());
        // encoder는 GPU에 보내는 명령들을 임시적으로 저장하는 것
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Encoder"),
        });

        // render_pass가 encoder를 빌려오기 때문에 아래처럼 따로 빼지 않으면 앞으로 계속 쓸 수 없음
        {
            let primitives = self.update_egui(&mut encoder);
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
                            a: 1.0
                        }),
                        // 처리한 색상을 위에서 지정한 view에 작성할지 말지 지정
                        // 우린 단색으로 도배하니 언제나 true로 설정
                        store: true
                    },
                })],
                // 깊이맵, 스텐실은 아직 안쓰니 None
                depth_stencil_attachment: None,
            });

            self.egui_renderer.render(&mut render_pass, &primitives, &self.egui_screen)
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
    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        let egui_response = self.egui_state.on_event(&self.egui_context, event);
        egui_response.consumed | false
    }

    fn update_egui(&mut self, encoder: &mut CommandEncoder) -> Vec<ClippedPrimitive> {
        let mut dummy = String::from("3489rty9843yur894uf");
        let egui_input = self.egui_state.take_egui_input(&self.window);
        let egui_output = self.egui_context.run(egui_input, |ctx| {
            eframe::egui::SidePanel::right("Side Menu")
                .resizable(true)
                .width_range(0.0..=512.0)
                .default_width(100.0)
                .show(ctx, |ui| {
                    TextEdit::singleline(&mut dummy).clip_text(false).desired_width(f32::INFINITY).show(ui);
                });
        });

        self.egui_state.handle_platform_output(&self.window, &self.egui_context, egui_output.platform_output);
        let primitives = self.egui_context.tessellate(egui_output.shapes);
        egui_output.textures_delta.set.iter().for_each(|(id, delta)| {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, &delta);
        });

        self.egui_renderer.update_buffers(&self.device, &self.queue, encoder, &primitives, &self.egui_screen);

        primitives
    }
}
