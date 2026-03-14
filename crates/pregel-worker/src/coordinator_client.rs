//! gRPC client for coordinator communication.

pub(crate) mod coordinator_proto {
    tonic::include_proto!("pregel.coordinator");
}

use coordinator_proto::coordinator_client::CoordinatorClient;
use coordinator_proto::{
    GetCurrentSuperstepRequest, RegisterWorkerRequest, ReportSuperstepDoneRequest,
    WaitForAllReadyRequest, WaitForJobStartRequest,
};
use pregel_common::WorkerId;
use std::time::Duration;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::Request;

/// Default timeout for coordinator gRPC calls. Prevents worker from hanging if coordinator is unreachable.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub struct CoordinatorGrpcClient {
    client: CoordinatorClient<Channel>,
}

impl CoordinatorGrpcClient {
    pub async fn connect(addr: String) -> Result<Self, tonic::transport::Error> {
        let client = CoordinatorClient::connect(addr).await?;
        Ok(Self { client })
    }

    pub async fn register_worker(
        &mut self,
        worker_id: WorkerId,
        address: String,
        vertex_count: u64,
    ) -> Result<(), tonic::Status> {
        self.client
            .register_worker(
                Request::new(RegisterWorkerRequest {
                    worker_id,
                    address,
                    vertex_count,
                })
                .timeout(REQUEST_TIMEOUT),
            )
            .await?;
        Ok(())
    }

    pub async fn wait_for_all_ready(&mut self) -> Result<(), tonic::Status> {
        self.client
            .wait_for_all_ready(Request::new(WaitForAllReadyRequest {}).timeout(REQUEST_TIMEOUT))
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
            .report_superstep_done(
                Request::new(ReportSuperstepDoneRequest {
                    worker_id,
                    superstep,
                    messages_sent,
                })
                .timeout(REQUEST_TIMEOUT),
            )
            .await?;
        Ok(())
    }

    pub async fn get_current_superstep(&mut self) -> Result<u64, tonic::Status> {
        let r = self
            .client
            .get_current_superstep(Request::new(GetCurrentSuperstepRequest {}).timeout(REQUEST_TIMEOUT))
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

    /// Session mode: block until coordinator starts a new job. Returns (job_id, algo, program).
    pub async fn wait_for_job_start(
        &mut self,
    ) -> Result<(u32, String, String, u64), tonic::Status> {
        let mut stream = self
            .client
            .wait_for_job_start(Request::new(WaitForJobStartRequest {}).timeout(REQUEST_TIMEOUT))
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
