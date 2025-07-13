//! gRPC server implementation for remote ExEx execution

pub mod server;
pub mod client;
pub mod health;
pub mod connection_pool;

#[cfg(feature = "grpc")]
pub mod proto {
    tonic::include_proto!("monmouth.exex.v1");
}

pub use server::ExExGrpcServer;
pub use client::ExExGrpcClient;
pub use health::HealthService;
pub use connection_pool::ConnectionPool;