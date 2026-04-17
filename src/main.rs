use std::rc::Rc;
use std::cell::Cell;
use std::sync::OnceLock;

use dpi::PhysicalSize;
use servo::{
    DeviceIntPoint, DeviceIntRect, DeviceIntSize, DevicePoint, RenderingContext, ServoBuilder,
    SoftwareRenderingContext, WebView, WebViewBuilder, WebViewDelegate,
    InputEvent, MouseButton as ServoMouseButton, MouseButtonAction, MouseButtonEvent,
    MouseMoveEvent, WheelDelta, WheelEvent, WheelMode, KeyboardEvent,
};
use url::Url;
use keyboard_types::{Key, KeyState, NamedKey};
use std::pin::Pin;
use std::future::Future;
use servo::protocol_handler::{
    DoneChannel, FetchContext, ProtocolHandler, ProtocolRegistry, Request, Response,
    ResponseBody, ResourceFetchTiming,
};
use headers::HeaderValue;

struct ResourceReader;

static RESOURCE_DIR: OnceLock<std::path::PathBuf> = OnceLock::new();
static RESOURCE_READER: ResourceReader = ResourceReader;

servo::submit_resource_reader!(&RESOURCE_READER);

impl servo::resources::ResourceReaderMethods for ResourceReader {
    fn read(&self, res: servo::resources::Resource) -> Vec<u8> {
        let resources_dir = RESOURCE_DIR
            .get()
            .expect("Resource directory not initialized");
        std::fs::read(resources_dir.join(res.filename())).unwrap_or_default()
    }
    fn sandbox_access_files_dirs(&self) -> Vec<std::path::PathBuf> {
        RESOURCE_DIR
            .get()
            .map(|path| vec![path.clone()])
            .unwrap_or_default()
    }
    fn sandbox_access_files(&self) -> Vec<std::path::PathBuf> {
        vec![]
    }
}

struct App {
    needs_repaint: Cell<bool>,
}

impl WebViewDelegate for App {
    fn handle_game_engine_spawn_enemy(&self, webview: WebView, enemy_id: String, x: f32, y: f32) {
        println!("Spawn {} at ({}, {})", enemy_id, x, y);
        webview.fire_gameengine_enemydied(enemy_id, x, y);
    }

    fn notify_new_frame_ready(&self, _webview: WebView) {
        self.needs_repaint.set(true);
    }

    fn notify_cursor_changed(&self, _webview: WebView, cursor: servo::Cursor) {
        println!("Cursor: {:?}", cursor);
    }
}

pub struct AppProtocolHandler {
    pub ui_root: std::path::PathBuf,
}

impl ProtocolHandler for AppProtocolHandler {
    fn is_fetchable(&self) -> bool { true }
    fn is_secure(&self) -> bool { true }

    fn load(
        &self,
        request: &mut Request,
        _done_chan: &mut DoneChannel,
        _context: &FetchContext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        let url = request.current_url();
        let path = self.ui_root.join(url.path().trim_start_matches('/'));

        let content = std::fs::read(&path).unwrap_or_else(|_| {
            format!("<html><body><h1>404: {}</h1></body></html>", path.display()).into_bytes()
        });

        let mime = match path.extension().and_then(|e| e.to_str()) {
            Some("html") => "text/html; charset=utf-8",
            Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("json") => "application/json",
            Some("svg") => "image/svg+xml",
            Some("png") => "image/png",
            Some("wasm") => "application/wasm",
            _ => "application/octet-stream",
        };

        let mut response = Response::new(url.clone(), ResourceFetchTiming::new(request.timing_type()));
        response.headers.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static(mime),
        );
        *response.body.lock() = ResponseBody::Done(content);

        Box::pin(std::future::ready(response))
    }
}

pub fn main() {
    let dev_mode = std::env::args().any(|a| a == "--dev");
    let resources_dir = std::path::PathBuf::from(
        std::env::var("SERVO_RESOURCE_PATH").unwrap_or_else(|_| "resources".to_string()),
    );
    RESOURCE_DIR
        .set(resources_dir)
        .expect("Resource directory already initialized");

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let width = 800u32;
    let height = 600u32;

    let window = video_subsystem
        .window("Servo SDL2", width, height)
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();
    let mut ui_texture = texture_creator
        .create_texture_streaming(sdl2::pixels::PixelFormatEnum::ABGR8888, width, height)
        .unwrap();

    let size = PhysicalSize::new(width, height);
    let render_ctx = Rc::new(SoftwareRenderingContext::new(size).unwrap());

    let mut protocol_registry = ProtocolRegistry::default();
    let ui_root = std::path::PathBuf::from(env!("SERVO_UI_DIST"));
    let _ = protocol_registry.register("app", AppProtocolHandler { ui_root });

    let servo = ServoBuilder::default().protocol_registry(protocol_registry).build();
    servo.setup_logging();

    let app = Rc::new(App { needs_repaint: Cell::new(false) });

    let webview = WebViewBuilder::new(&servo, render_ctx.clone())
        .delegate(app.clone())
        .url(if dev_mode {
            Url::parse("http://localhost:5173").unwrap()
        } else {
            Url::parse("app://main/index.html").unwrap()
        })
        .build();

    webview.focus();
    video_subsystem.text_input().start();
    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit { .. } => break 'running,
                sdl2::event::Event::Window { win_event: sdl2::event::WindowEvent::Resized(w, h), .. } => {
                    let new_size = PhysicalSize::new(w as u32, h as u32);
                    webview.resize(new_size);
                    ui_texture = texture_creator
                        .create_texture_streaming(sdl2::pixels::PixelFormatEnum::ABGR8888, w as u32, h as u32)
                        .unwrap();
                }
                sdl2::event::Event::MouseMotion { x, y, .. } => {
                    let point = DevicePoint::new(x as f32, y as f32);
                    webview.notify_input_event(InputEvent::MouseMove(MouseMoveEvent::new(point.into())));
                }
                sdl2::event::Event::MouseButtonDown { mouse_btn, x, y, .. } => {
                    let point = DevicePoint::new(x as f32, y as f32);
                    let button = sdl_mouse_to_servo(mouse_btn);
                    webview.notify_input_event(InputEvent::MouseButton(MouseButtonEvent::new(
                        MouseButtonAction::Down, button, point.into(),
                    )));
                }
                sdl2::event::Event::MouseButtonUp { mouse_btn, x, y, .. } => {
                    let point = DevicePoint::new(x as f32, y as f32);
                    let button = sdl_mouse_to_servo(mouse_btn);
                    webview.notify_input_event(InputEvent::MouseButton(MouseButtonEvent::new(
                        MouseButtonAction::Up, button, point.into(),
                    )));
                }
                sdl2::event::Event::MouseWheel { x, y, .. } => {
                    let delta = WheelDelta { x: x as f64 * 38.0, y: y as f64 * 38.0, z: 0.0, mode: WheelMode::DeltaPixel };
                    webview.notify_input_event(InputEvent::Wheel(WheelEvent::new(delta, DevicePoint::new(0.0, 0.0).into())));
                }
                sdl2::event::Event::TextInput { text, .. } => {
                    let key = Key::Character(text);
                    webview.notify_input_event(InputEvent::Keyboard(KeyboardEvent::from_state_and_key(KeyState::Down, key.clone())));
                    webview.notify_input_event(InputEvent::Keyboard(KeyboardEvent::from_state_and_key(KeyState::Up, key)));
                }
                sdl2::event::Event::KeyDown { keycode: Some(kc), .. } => {
                    if let Some(key) = sdl_key(kc, &webview) {
                        webview.notify_input_event(InputEvent::Keyboard(KeyboardEvent::from_state_and_key(KeyState::Down, key)));
                    }
                }
                sdl2::event::Event::KeyUp { keycode: Some(kc), .. } => {
                    if let Some(key) = sdl_key(kc, &webview) {
                        webview.notify_input_event(InputEvent::Keyboard(KeyboardEvent::from_state_and_key(KeyState::Up, key)));
                    }
                }
                _ => {}
            }
        }

        servo.spin_event_loop();

        if app.needs_repaint.get() {
            app.needs_repaint.set(false);
            webview.paint();

            let (cw, ch) = canvas.output_size().unwrap();
            let rect = DeviceIntRect::from_origin_and_size(
                DeviceIntPoint::new(0, 0),
                DeviceIntSize::new(cw as i32, ch as i32),
            );

            if let Some(image) = render_ctx.read_to_image(rect) {
                ui_texture.update(None, image.as_raw(), (cw * 4) as usize).unwrap();
            }
        }

        canvas.clear();
        canvas.copy(&ui_texture, None, None).unwrap();
        canvas.present();
    }
}

fn sdl_mouse_to_servo(btn: sdl2::mouse::MouseButton) -> ServoMouseButton {
    match btn {
        sdl2::mouse::MouseButton::Left => ServoMouseButton::Left,
        sdl2::mouse::MouseButton::Right => ServoMouseButton::Right,
        sdl2::mouse::MouseButton::Middle => ServoMouseButton::Middle,
        sdl2::mouse::MouseButton::X1 => ServoMouseButton::Back,
        sdl2::mouse::MouseButton::X2 => ServoMouseButton::Forward,
        _ => ServoMouseButton::Other(0),
    }
}

fn sdl_key(kc: sdl2::keyboard::Keycode, webview: &WebView) -> Option<Key> {
    match kc {
        sdl2::keyboard::Keycode::Backspace => Some(Key::Named(NamedKey::Backspace)),
        sdl2::keyboard::Keycode::Delete => Some(Key::Named(NamedKey::Delete)),
        sdl2::keyboard::Keycode::Return => Some(Key::Named(NamedKey::Enter)),
        sdl2::keyboard::Keycode::Escape => Some(Key::Named(NamedKey::Escape)),
        sdl2::keyboard::Keycode::Up => Some(Key::Named(NamedKey::ArrowUp)),
        sdl2::keyboard::Keycode::Down => Some(Key::Named(NamedKey::ArrowDown)),
        sdl2::keyboard::Keycode::Left => Some(Key::Named(NamedKey::ArrowLeft)),
        sdl2::keyboard::Keycode::Right => Some(Key::Named(NamedKey::ArrowRight)),
        sdl2::keyboard::Keycode::F5 => { webview.reload(); None }
        _ => None,
    }
}
