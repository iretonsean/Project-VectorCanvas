fn main() {
    let inst = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::all(), ..wgpu::InstanceDescriptor::new_without_display_handle() });
    for a in pollster_block(inst.enumerate_adapters(wgpu::Backends::all())) {
        let i = a.get_info();
        let d = a.get_downlevel_capabilities();
        println!("{:?} | {} | {:?} | driver={} {} | compute={}", i.backend, i.name, i.device_type, i.driver, i.driver_info, d.flags.contains(wgpu::DownlevelFlags::COMPUTE_SHADERS));
    }
}
fn pollster_block<F: std::future::Future>(f: F) -> F::Output {
    use std::task::*; let w = Waker::noop(); let mut c = Context::from_waker(&w); let mut f = std::pin::pin!(f);
    loop { if let Poll::Ready(v) = f.as_mut().poll(&mut c) { return v; } }
}
