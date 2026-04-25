mod renderer;
mod runtime;

use std::num::NonZeroU32;
use std::path::Path;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event::{ElementState, MouseButton};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
use winit::window::{Window, WindowAttributes};

const SVG_PATH: &str = "vbr_ui.svg";

struct App {
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<OwnedDisplayHandle, Rc<Window>>>,
    context: Option<softbuffer::Context<OwnedDisplayHandle>>,
    runtime: runtime::VbrRuntime,
    renderer: renderer::Renderer,
    /// Last known cursor position (SVG/physical coordinates)
    cursor_pos: PhysicalPosition<f64>,
}

impl App {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let runtime = runtime::VbrRuntime::new(Path::new(SVG_PATH))?;
        let (w, h) = runtime.dimensions();
        let renderer = renderer::Renderer::new(w, h);

        Ok(Self {
            window: None,
            surface: None,
            context: None,
            runtime,
            renderer,
            cursor_pos: PhysicalPosition { x: 0.0, y: 0.0 },
        })
    }

    fn render_frame(&self) -> Result<tiny_skia::Pixmap, Box<dyn std::error::Error>> {
        self.renderer.render_svg(self.runtime.svg_data())
    }

    fn present_frame(&mut self, dirty: &[runtime::DirtyRect]) {
        if dirty.is_empty() {
            return;
        }

        let pixmap = match self.render_frame() {
            Ok(p) => p,
            Err(_) => {
                eprintln!("render_frame failed");
                return;
            }
        };
        let pix_data = pixmap.data(); // RGBA premultiplied

        let Some(ref mut surface) = self.surface else {
            return;
        };

        let (w, h) = self.runtime.dimensions();
        let non_zero_w = NonZeroU32::new(w).unwrap();
        let non_zero_h = NonZeroU32::new(h).unwrap();
        if surface.resize(non_zero_w, non_zero_h).is_err() {
            eprintln!("surface.resize failed");
            return;
        }

        if let Ok(mut buffer) = surface.buffer_mut() {
            let bw = buffer.width().get() as usize;
            let bh = buffer.height().get() as usize;

            for r in dirty {
                let x0 = (r.x as usize).min(bw);
                let y0 = (r.y as usize).min(bh);
                let x1 = ((r.x + r.w) as usize).min(bw);
                let y1 = ((r.y + r.h) as usize).min(bh);
                if x1 <= x0 || y1 <= y0 {
                    continue;
                }

                for y in y0..y1 {
                    for x in x0..x1 {
                        let i = y * bw + x;
                        let p = i * 4;
                        if p + 3 >= pix_data.len() {
                            continue;
                        }
                        let b = pix_data[p + 2] as u32;
                        let g = pix_data[p + 1] as u32;
                        let r = pix_data[p] as u32;
                        let a = pix_data[p + 3] as u32;
                        buffer[i] = b | (g << 8) | (r << 16) | (a << 24);
                    }
                }
            }

            buffer.present().ok();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let (w, h) = self.runtime.dimensions();
        let window_attrs = WindowAttributes::default()
            .with_title("VBR UI")
            .with_inner_size(PhysicalSize::new(w, h))
            .with_resizable(false);

        let window = Rc::new(
            event_loop
                .create_window(window_attrs)
                .expect("failed to create window"),
        );

        // Context may already exist (created before run_app)
        let context = self
            .context
            .get_or_insert_with(|| {
                softbuffer::Context::new(event_loop.owned_display_handle())
                    .expect("softbuffer context")
            });

        let surface = softbuffer::Surface::new(context, window.clone())
            .expect("softbuffer surface");

        self.surface = Some(surface);
        self.window = Some(window);

        let dirty = self.runtime.take_dirty_rects();
        self.present_frame(&dirty);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.runtime.tick_animation();
                let dirty = self.runtime.take_dirty_rects();
                if !dirty.is_empty() {
                    let touched: u64 = dirty.iter().map(|r| (r.w as u64) * (r.h as u64)).sum();
                    self.present_frame(&dirty);
                    println!("PERF: dirty_rects={} pixels={}", dirty.len(), touched);
                }
                if self.runtime.is_animating() {
                    if let Some(ref w) = self.window {
                        w.request_redraw();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = position;
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let (svg_w, svg_h) = self.runtime.dimensions();
                let (mapped_x, mapped_y) = if let Some(ref w) = self.window {
                    let win = w.inner_size();
                    if win.width > 0 && win.height > 0 {
                        let sx = svg_w as f64 / win.width as f64;
                        let sy = svg_h as f64 / win.height as f64;
                        (self.cursor_pos.x * sx, self.cursor_pos.y * sy)
                    } else {
                        (self.cursor_pos.x, self.cursor_pos.y)
                    }
                } else {
                    (self.cursor_pos.x, self.cursor_pos.y)
                };

                let msg = self.runtime.handle_click(mapped_x, mapped_y);
                println!("{}", msg);

                if let Some(ref w) = self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.runtime.handle_mouse_up();
                if let Some(ref w) = self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new()?;

    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut app)?;

    Ok(())
}
