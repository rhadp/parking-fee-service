pub mod broker;
pub mod config;
pub mod event_loop;
pub mod grpc_server;
pub mod operator;
pub mod session;

#[cfg(test)]
pub mod testing;
#[cfg(test)]
pub mod proptest_cases;

/// Re-export generated protobuf types for internal use.
pub mod proto {
    pub mod kuksa {
        pub mod val {
            pub mod v2 {
                tonic::include_proto!("kuksa.val.v2");
            }
        }
    }
    pub mod parking_adaptor {
        pub mod v1 {
            tonic::include_proto!("parking_adaptor.v1");
        }
    }
}
