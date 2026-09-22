//! OTLP gRPC collector services (trace, metrics, logs).

use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::convert;
use crate::db::Db;
use crate::proto::opentelemetry::proto::collector::{
    logs::v1::logs_service_server::{LogsService, LogsServiceServer},
    metrics::v1::metrics_service_server::{MetricsService, MetricsServiceServer},
    trace::v1::trace_service_server::{TraceService, TraceServiceServer},
};

pub struct OtlpCollector {
    pub db: Arc<Db>,
}

macro_rules! export_impl {
    ($trait_name:ident, $req:ty, $resp:ty, $convert:ident, $insert:ident) => {
        #[tonic::async_trait]
        impl $trait_name for OtlpCollector {
            async fn export(&self, request: Request<$req>) -> Result<Response<$resp>, Status> {
                let req = request.into_inner();
                let rows = convert::$convert(&req);
                self.db
                    .$insert(rows)
                    .await
                    .map_err(|e| Status::internal(format!("failed to store data: {e}")))?;
                Ok(Response::new(<$resp as Default>::default()))
            }
        }
    };
}

export_impl!(
    TraceService,
    crate::proto::opentelemetry::proto::collector::trace::v1::ExportTraceServiceRequest,
    crate::proto::opentelemetry::proto::collector::trace::v1::ExportTraceServiceResponse,
    trace_request,
    insert_spans
);

export_impl!(
    LogsService,
    crate::proto::opentelemetry::proto::collector::logs::v1::ExportLogsServiceRequest,
    crate::proto::opentelemetry::proto::collector::logs::v1::ExportLogsServiceResponse,
    logs_request,
    insert_logs
);

export_impl!(
    MetricsService,
    crate::proto::opentelemetry::proto::collector::metrics::v1::ExportMetricsServiceRequest,
    crate::proto::opentelemetry::proto::collector::metrics::v1::ExportMetricsServiceResponse,
    metrics_request,
    insert_metrics
);

const MAX_MSG: usize = 32 * 1024 * 1024;

/// Build the tonic gRPC server router for all three OTLP services.
pub fn router(db: Arc<Db>) -> tonic::transport::server::Router {
    use tonic::codec::CompressionEncoding::Gzip;
    tonic::transport::Server::builder()
        .add_service(
            TraceServiceServer::new(OtlpCollector { db: db.clone() })
                .accept_compressed(Gzip)
                .max_decoding_message_size(MAX_MSG),
        )
        .add_service(
            MetricsServiceServer::new(OtlpCollector { db: db.clone() })
                .accept_compressed(Gzip)
                .max_decoding_message_size(MAX_MSG),
        )
        .add_service(
            LogsServiceServer::new(OtlpCollector { db })
                .accept_compressed(Gzip)
                .max_decoding_message_size(MAX_MSG),
        )
        .add_service(
            tonic_reflection::server::Builder::configure()
                .register_encoded_file_descriptor_set(crate::proto::FILE_DESCRIPTOR_SET)
                .build_v1()
                .unwrap(),
        )
}
