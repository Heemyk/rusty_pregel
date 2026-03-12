//! gRPC client for coordinator communication.

pub(crate) mod coordinator_proto {
    tonic::include_proto!("pregel.coordinator");
}

use coordinator_proto::coordinator_client::CoordinatorClient;
use coordinator_proto::{
    GetCurrentSuperstepRequest, RegisterWorkerRequest, ReportSuperstepDoneRequest,
    WaitForAllReadyRequest,
};
use pregel_common::WorkerId;
use tonic::transport::Channel;
use tonic::Request;


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
}
