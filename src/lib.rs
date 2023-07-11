use cfg_if::cfg_if;
use nalgebra::Vector4;
use wgpu::SurfaceError;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Icon, WindowBuilder};

cfg_if! {
    // 만약 현재 환경이 wasm32라면
    if #[cfg(target_arch = "wasm32")] {
        // web time 사용
        use web_time::{SystemTime, UNIX_EPOCH};
    } else {
        // 네이티브 time 사용
        use std::time::{SystemTime, UNIX_EPOCH};
    }
}

// wasm32 환경에서만 wasm_bindgen 활용
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::app::Application;

mod app;
mod lantern;
mod camera;

// wasm 연결시 아래 함수를 시작점으로 삼도록 함.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    // 로거 초기화
    cfg_if! {
        // 만약 현재 환경이 wasm32라면
        if #[cfg(target_arch = "wasm32")] {
            // panic 발생시 웹 브라우저의 console.err에 로그 띄우기
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("로거 초기화 실패");
        } else {
            // 아니면 기본적인 로거만 불러오기
            env_logger::init();
        }
    }

    // 아이콘 불러오기
    let icon = {
        // 실행 파일에 아이콘 이미지 포함
        let bytes = include_bytes!("../sun.png");
        let image = image::load_from_memory(bytes).unwrap();
        let width = image.width();
        let height = image.height();

        let rgba = image.into_rgba8();

        Icon::from_rgba(
            rgba.into_vec(),
            width,
            height,
        )
    };

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_window_icon(icon.ok())
        .with_title("Lantern: Ray Tracer")
        .build(&event_loop)
        .unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // 캔버스 문제 해결
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }


    let mut app = Application::new(window, &event_loop).await;
    let mut last_frame_time = now();

    event_loop.run(move |event, _target, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == app.window.id() && !app.input(event) => match event {
            // 만약 앱을 운영체제에서 닫으려고 하거나
            WindowEvent::CloseRequested |
            // 키보드 입력이 들어왔고
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    // 키보드가 새로 눌러졌으며, 그 눌러진 키가 ESC라면
                    state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Escape), ..
                }, ..
            } => *control_flow = ControlFlow::ExitWithCode(0), // 나가기
            WindowEvent::Resized(size) => app.resize(*size),
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => app.resize(**new_inner_size),
            _ => {}
        },
        Event::RedrawRequested(window_id) if window_id == app.window.id() => {
            let now = now();
            let frame_time = now - last_frame_time;

            app.update(frame_time);
            match app.render(frame_time) {
                Ok(_) => {},
                // 모종의 이유로 swap chain이 깨지면 surface를 재구성하기.
                Err(SurfaceError::Lost) => app.resize(app.size),
                // 메모리 부족시 그냥 -1로 튕기기
                Err(SurfaceError::OutOfMemory) => *control_flow = ControlFlow::ExitWithCode(-1),

                // Outdated, Timeout은 그냥 다음 프레임때면 알아서 고쳐지니 출력만 하고 아무것도 하지 말기
                Err(e) => eprintln!("{:?}", e),
            }

            last_frame_time = now;
        }
        // window.request_redraw는 앱 시작시 원랜 한번만 실행됨
        // 그러나 우린 실시간 렌더링 앱을 만들기에 계속 다시 그려야함
        // 그래서 MainEvent가 다 비어서 이제 할꺼 없으면 바로 다시 그리도록 하기
        Event::MainEventsCleared => {
            app.window.request_redraw()
        }
        _ => {}
    });
}

fn vec4_to_rgba(vec4: &Vector4<f32>) -> u32 {
    let red = (vec4.x * 255.0) as u32;
    let green = (vec4.y * 255.0) as u32;
    let blue = (vec4.z * 255.0) as u32;
    let alpha = (vec4.w * 255.0) as u32;

    (alpha << 24) | (blue << 16) | (green << 8) | red
}

fn now() -> u128 {
    let current = SystemTime::now();
    let since_epoch = current.duration_since(UNIX_EPOCH).unwrap();

    since_epoch.as_millis()
}

