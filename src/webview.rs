#![deny(warnings)]
use wry::WebViewBuilder;
use tao::{
    dpi::{LogicalSize},
    event::{Event, WindowEvent,StartCause},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{WindowBuilder,Icon}
};

pub struct Webview{}

impl Webview{
    pub fn initialize(address: &String, width: u16, height: u16, title: &String,imagedata: Vec<u8>) -> wry::Result<()>{
        use tao::platform::windows::EventLoopBuilderExtWindows;
        let icon = match Icon::from_rgba(imagedata,100,100){
            Ok(i) => Some(i),
            Err(e) => {eprintln!("{e}");None}
        };
        let event_loop = EventLoopBuilder::new().with_any_thread(false).build();
        let window_size = LogicalSize::new(width,height);
        let window = WindowBuilder::new()
            .with_inner_size(window_size)
            .with_title(title)
            .with_window_icon(icon)
            .build(&event_loop).unwrap();
        println!("Webview should load: '{}'",address);
        let builder = WebViewBuilder::new().with_url(address);

        #[cfg(not(target_os = "linux"))]
        let _webview = builder.build(&window).unwrap();
        #[cfg(target_os = "linux")]
        let _webview = builder.build_gtk(window.gtk_window()).unwrap();
    
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
        
            match event {
                Event::NewEvents(StartCause::Init) => println!("Wry has started!"),
                Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
                } => {
                    *control_flow = ControlFlow::Exit;
                },
                _ => (),
            }
        });
    }
}
