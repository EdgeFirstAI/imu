use zmq::{Context, Socket};

// Source can be found in: https://github.com/erickt/rust-zmq/blob/master/examples/zguide/helloworld_server/main.rs
// IMU Application Server
// Binds PUB socket to tcp://*:5556
// Publishes random yaw, pitch, roll values.
pub struct Server {
    pub endpoint: String,
    pub publisher: Socket
}

impl Server {
    pub fn new(endpoint: String) -> Self {
        let context = Context::new();
        let publisher = context.socket(zmq::PUB).unwrap();
        Self { 
            endpoint,
            publisher
        }
    }

    pub fn start_server(&self) {
        assert!(self.publisher.bind(&self.endpoint).is_ok());
    }
}
