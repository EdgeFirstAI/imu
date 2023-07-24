#![crate_name = "camerapose"]
mod server;

use crate::server::Server;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "IMU Application", about = "Pushes IMU angles to the endpoint set.")]
struct Opt {
    #[structopt(short = "e", long = "endpoint", help = "Set the endpoint to push data", default_value = "ipc:///tmp/pose.pub")]
    endpoint: String,
}

fn main() {
    let opt = Opt::from_args();
    let server = Server {
        endpoint: opt.endpoint,
    };
    server.start_server();
}