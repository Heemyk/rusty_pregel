//! gRPC client for coordinator communication.

pub(crate) mod coordinator_proto {
    tonic::include_proto!("pregel.coordinator");
}

use coordinator_proto::coordinator_client::CoordinatorClient;
use coordinator_proto::{
    GetCurrentSuperstepRequest, RegisterWorkerRequest, ReportJobResultsRequest,
    ReportSuperstepDoneRequest, VertexResult, WaitForAllReadyRequest, WaitForJobStartRequest,
};
use pregel_common::WorkerId;
use std::time::Duration;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::Request;

/// Default timeout for coordinator gRPC calls.
/// Long timeout allows wait_for_job_start to block for next job in session mode.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(86400); // 24h

pub struct CoordinatorGrpcClient {
    client: CoordinatorClient<Channel>,
}

impl CoordinatorGrpcClient {
    pub async fn connect(addr: String) -> Result<Self, tonic::transport::Error> {
        let channel = tonic::transport::Endpoint::from_shared(addr)?
            .timeout(REQUEST_TIMEOUT)
            .connect()
            .await?;
        let client = CoordinatorClient::new(channel);
        Ok(Self { client })
    }

    pub async fn register_worker(
        &mut self,
        worker_id: WorkerId,
        address: String,
        vertex_count: u64,
    ) -> Result<(), tonic::Status> {
        self.client
            .register_worker(Request::new(RegisterWorkerRequest {
                worker_id,
                address,
                vertex_count,
            }))
            .await?;
        Ok(())
    }

    pub async fn wait_for_all_ready(&mut self) -> Result<(), tonic::Status> {
        self.client
            .wait_for_all_ready(Request::new(WaitForAllReadyRequest {}))
            .await?;
        Ok(())
    }

    pub async fn report_superstep_done(
        &mut self,
        worker_id: WorkerId,
        superstep: u64,
        messages_sent: u64,
    ) -> Result<(), tonic::Status> {
        self.client
            .report_superstep_done(Request::new(ReportSuperstepDoneRequest {
                worker_id,
                superstep,
                messages_sent,
            }))
            .await?;
        Ok(())
    }

    pub async fn get_current_superstep(&mut self) -> Result<u64, tonic::Status> {
        let r = self
            .client
            .get_current_superstep(Request::new(GetCurrentSuperstepRequest {}))
            .await?
            .into_inner();
        Ok(r.superstep)
    }

    /// Block until coordinator advances past from_superstep. Returns the new superstep.
    /// Uses polling (1ms) - streaming WaitForAdvance hangs on fresh coordinator start.
    pub async fn wait_for_advance(&mut self, from_superstep: u64) -> Result<u64, tonic::Status> {
        loop {
            let step = self.get_current_superstep().await?;
            if step > from_superstep {
                return Ok(step);
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    }

    /// Report job results (vertex values) when halting. Coordinator aggregates and sends to HTTP client.
    pub async fn report_job_results(
        &mut self,
        job_id: u32,
        worker_id: u32,
        vertices: Vec<(u64, Vec<u8>)>,
    ) -> Result<(), tonic::Status> {
        let vertices: Vec<VertexResult> = vertices
            .into_iter()
            .map(|(vertex_id, value)| VertexResult { vertex_id, value })
            .collect();
        self.client
            .report_job_results(Request::new(ReportJobResultsRequest {
                job_id,
                worker_id,
                vertices,
            }))
            .await?;
        Ok(())
    }

    /// Session mode: block until coordinator starts a new job. Returns (job_id, algo, program).
    pub async fn wait_for_job_start(
        &mut self,
    ) -> Result<(u32, String, String, u64), tonic::Status> {
        let mut stream = self
            .client
            .wait_for_job_start(Request::new(WaitForJobStartRequest {}))
            .await?
            .into_inner();
        let msg = stream
            .next()
            .await
            .transpose()?
            .ok_or_else(|| tonic::Status::unavailable("job start stream closed"))?;
        Ok((
            msg.job_id,
            msg.algo,
            msg.program,
            msg.total_vertices,
        ))
    }
}
