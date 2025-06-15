#![deny(warnings)]
use std::net::SocketAddr;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request};
use tokio::net::TcpListener;

use tokio_util::sync::CancellationToken;

// This would normally come from the `hyper-util` crate, but we can't depend
// on that here because it would be a cyclical dependency.
use crate::support::{TokioIo, TokioTimer};

use crate::settings::Settings;
use crate::service_response::ServiceResponse;
use crate::server_service::ServerCommand;
use crate::httpsconnector::{RequestOptions,request_json};
use crate::models::RemoteData;
use crate::RuntimeMode;

pub type TaskResult = Result<TaskInfo, TaskError>;

pub enum ServerTask{ 
    Update,
    Start,
}

#[derive(Debug)]
pub enum TaskError{
    Unavailable,
    NotFound,
    Failure,
    InvalidResource
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            TaskError::Unavailable=> write!(f, "Not available"),
            TaskError::NotFound => write!(f, "Update resource not found"),
            TaskError::Failure => write!(f, "Task failed majestically"),
            TaskError::InvalidResource => write!(f,"Update resource couldn't be deserialized")
        }
    }
}

pub struct TaskInfo{
    task_kind: ServerTask,
    task_data: Option<RemoteData>
}

impl TaskInfo{
    pub fn data(self) -> Result<RemoteData,TaskError>{
        match self.task_kind{
            ServerTask::Update => Ok(self.task_data.expect("No data!")),
            _ => Err(TaskError::Unavailable)
        }
    }
}

impl std::process::Termination for TaskInfo{
    fn report(self) -> std::process::ExitCode {
        std::process::ExitCode::SUCCESS
    }
}


async fn shutdown_signal(token: CancellationToken) {
    token.cancelled().await
}

#[tokio::main]
pub async fn update_task(conf: &Settings, runtime_mode: RuntimeMode) -> TaskResult{
    let resource = match conf.get_resource("update") {
        Some(res) => res,
        None => return Err(TaskError::Failure)
    };
    let creds = match resource.request_credentials("2.718281828459045"){
        Some(res) => match res{
            Ok(dec) => Some(dec),
            Err(_) => return Err(TaskError::InvalidResource)
        },
        None => None
    };
    let request_init = RequestOptions{
        uri: resource.uri.uri().clone(),
        credentials: creds,
        user_agent: &conf.user_agent,
        model: resource.model.clone(),
        schema: conf.get_schema(&resource.schema)
    };
    let result = match request_json(request_init).await{
        Ok(r) => match &resource.target{
            Some(res) => {
                match runtime_mode{
                    RuntimeMode::Normal => match res.write_file(&r).await{
                        Ok(_) => println!("file saved!"),
                        Err(e) => {
                            eprintln!("{:?}",e);
                            return Err(TaskError::InvalidResource)
                        }
                    },
                    _ => ()
                }
                r
            },
            None => r
        },
        Err(e) => {
          eprintln!("{:?}",e);
          return Err(TaskError::NotFound)
        }
      };
      Ok(TaskInfo{task_kind: ServerTask::Update, task_data: Some(result)})
}

#[tokio::main]
pub async fn start_server(conf: &Settings,runtime_mode: RuntimeMode) -> TaskResult {
    match conf.resources {
        None => println!("Server hosting content at './'"),
        Some(_) => println!("Server hosting content at './{}/'",conf.server_root)
    };
    // This address is localhost
    let addr: SocketAddr = ([127, 0, 0, 1], conf.port).into();
    
    // Bind to the port and listen for incoming TCP connections
    let listener = match TcpListener::bind(addr).await{
        Ok(it) => it,
        Err(e) =>  {
            eprintln!("{:?}",e);
            return Err(TaskError::Failure)
        } 
    };
    
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();
    // when this signal completes, start shutdown
    let token = CancellationToken::new();

    let mut signal = std::pin::pin!(shutdown_signal(token.clone()));
    
    
    match runtime_mode {
        RuntimeMode::Headless => (),
        RuntimeMode::Webview(args) => {
            let token_clone = token.clone();
            let mut exe_path = match std::env::current_exe(){
                Ok(path) => path,
                Err(e) => panic!("failed to get current exe path: {e}")
            };
            exe_path.pop();
            exe_path.push("webview-host.exe");
            println!("Path of this executable is: {}", exe_path.display());
            let address : String = match &conf.start_in{
                Some(s) => format!("http://localhost:{}/{}",addr.port(),s),
                None => format!("http://localhost:{}",addr.port())
            };
            let width_str : String = args.width.to_string();
            let height_str : String = args.height.to_string();

            tokio::spawn(async move {
                
                let mut child = match std::process::Command::new(exe_path.into_os_string())
                    .arg("--url")
                    .arg(address)
                    .arg("--width")
                    .arg(width_str)
                    .arg("--height")
                    .arg(height_str)
                    .arg("--title")
                    .arg(args.title.unwrap_or("Rusty webview".to_string()))
                    .spawn(){
                        Ok(c) => c,
                        Err(e) => panic!("Failed to spawn webview-host: {e}")
                    };
                child.wait().expect("webview-host didn't actually run");
                token_clone.cancel();
            });
            ()
        },
        RuntimeMode::Normal => {
            let address : String = match &conf.start_in{
                Some(s) => format!("http://localhost:{}/{}",addr.port(),s),
                None => format!("http://localhost:{}",addr.port())
            };
            tokio::spawn(async move {
                match webbrowser::open(&address){
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("{}",e);
                    }
                }
            });
            ()
        }
    };
    println!("Listening on http://{}", addr);
    loop {
        
        // When an incoming TCP connection is received grab a TCP stream for
        // client<->server communication.
        //
        // Note, this is a .await point, this loop will loop forever but is not a busy loop. The
        // .await point allows the Tokio runtime to pull the task off of the thread until the task
        // has work to do. In this case, a connection arrives on the port we are listening on and
        // the task is woken up, at which point the task is then put back on a thread, and is
        // driven forward by the runtime, eventually yielding a TCP stream.

        tokio::select!{
            Ok((stream,_addr)) = listener.accept() => {
                // Use an adapter to access something implementing `tokio::io` traits as if they implement
                // `hyper::rt` IO traits.
                let io = TokioIo::new(stream);
                // Spin up a new task in Tokio so we can continue to listen for new TCP connection on the
                // current task without waiting for the processing of the HTTP1 connection we just received
                // to finish
                let token_clone = token.clone();
                let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                    if token_clone.is_cancelled(){
                        return ServiceResponse::ServiceUnavailable.resolve()
                    }
                    let response = match req.method() {
                        &Method::GET => match req.uri().path().strip_prefix("/api/"){
                          Some(command) => match ServerCommand::from_str(command) {
                            Some(c) => ServiceResponse::CommandResponse(c),
                            None => ServiceResponse::NotFound,
                          },
                          None => ServiceResponse::FileService(req)
                        },
                        &Method::HEAD => match req.uri().path() {
                            "/api/shutdown" => {
                                token_clone.cancel();
                                ServiceResponse::Accepted
                            },
                            _ => ServiceResponse::NotFoundEmpty
                        },
                        &Method::POST => ServiceResponse::PostAPIResponse(req),
                        _ => ServiceResponse::BadMethod
                    };
                    response.resolve()
                    
                });
                let conn = http1::Builder::new()
                    .header_read_timeout(std::time::Duration::from_secs(5))
                    .timer(TokioTimer)
                    .serve_connection(io, service);
                let future = graceful.watch(conn);
                tokio::spawn(async move {
                    if let Err(err) = future.await{
                        if !err.is_timeout(){
                            println!("Error serving connection: {:?}", err);
                        }
                        
                    }
                });
            },
            _ = &mut signal => {
                eprintln!("graceful shutdown signal received");
                // stop the accept loop
                break;
            }
        }
        
    }
    
    // Now start the shutdown and wait for them to complete
    // Optional: start a timeout to limit how long to wait.

    tokio::select! {
        _ = graceful.shutdown() => {
            eprintln!("all connections gracefully closed");
        },
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
            eprintln!("timed out wait for all connections to close");
        }
    }
    Ok(TaskInfo{task_data: None, task_kind: ServerTask::Start})
}

