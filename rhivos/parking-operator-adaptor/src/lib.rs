pub mod broker;
pub mod config;
pub mod event_loop;
pub mod grpc_server;
pub mod operator;
pub mod session;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod proptest_cases;

pub mod proto {
    pub mod parking_adaptor {
        tonic::include_proto!("parking_adaptor");
    }
    pub mod kuksa {
        tonic::include_proto!("kuksa.val.v2");
    }
}
