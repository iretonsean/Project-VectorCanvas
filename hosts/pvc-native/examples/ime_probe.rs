//! IME probe: bare winit 0.30.13 window, logs every Ime/Keyboard event to stderr as `EV ...`.
//! No pvc code. Used to find out whether a *system* IME (fcitx5/ibus via XIM) talks to winit under Xvfb.
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

struct App { w: Option<Window> }
impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        let w = el.create_window(Window::default_attributes().with_title("ime_probe").with_inner_size(LogicalSize::new(800.0, 400.0))).unwrap();
        w.set_ime_allowed(true);
        w.set_ime_cursor_area(LogicalPosition::new(100.0, 100.0), LogicalSize::new(2.0, 20.0));
        eprintln!("EV window-created ime_allowed=true");
        self.w = Some(w);
    }
    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Ime(i) => eprintln!("EV Ime {:?}", i),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => eprintln!("EV Key logical={:?} text={:?}", event.logical_key, event.text),
            WindowEvent::Focused(f) => eprintln!("EV Focused {f}"),
            WindowEvent::RedrawRequested => {}
            _ => {}
        }
    }
}
fn main() {
    let el = EventLoop::new().unwrap();
    el.set_control_flow(ControlFlow::Wait);
    let mut a = App { w: None };
    el.run_app(&mut a).unwrap();
}
