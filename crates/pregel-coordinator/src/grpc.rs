//! gRPC server for the coordinator.

pub(crate) mod coordinator_proto {
    tonic::include_proto!("pregel.coordinator");
}

use coordinator_proto::coordinator_server::{Coordinator as CoordinatorTrait, CoordinatorServer};
use coordinator_proto::{
    AdvanceNotification, GetCurrentSuperstepRequest, GetCurrentSuperstepResponse,
    RegisterWorkerRequest, RegisterWorkerResponse, ReportSuperstepDoneRequest,
    ReportSuperstepDoneResponse, WaitForAdvanceRequest, WaitForAllReadyRequest,
    WaitForAllReadyResponse,
};
use pregel_common::WorkerId;
use pregel_core::Superstep;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{watch, Notify, RwLock};
use tonic::{Request, Response, Status};


struct ArcCoordinator(pub Arc<CoordinatorService>);

#[tonic::async_trait]
impl CoordinatorTrait for ArcCoordinator {
    async fn register_worker(
        &self,
        request: Request<RegisterWorkerRequest>,
    ) -> Result<Response<RegisterWorkerResponse>, Status> {
        CoordinatorTrait::register_worker(self.0.as_ref(), request).await
    }
    async fn wait_for_all_ready(
        &self,
        request: Request<WaitForAllReadyRequest>,
    ) -> Result<Response<WaitForAllReadyResponse>, Status> {
        CoordinatorTrait::wait_for_all_ready(self.0.as_ref(), request).await
    }
    async fn report_superstep_done(
        &self,
        request: Request<ReportSuperstepDoneRequest>,
    ) -> Result<Response<ReportSuperstepDoneResponse>, Status> {
        CoordinatorTrait::report_superstep_done(self.0.as_ref(), request).await
    }
    async fn get_current_superstep(
        &self,
        request: Request<GetCurrentSuperstepRequest>,
    ) -> Result<Response<GetCurrentSuperstepResponse>, Status> {
        CoordinatorTrait::get_current_superstep(self.0.as_ref(), request).await
    }
    type WaitForAdvanceStream = tokio_stream::wrappers::ReceiverStream<Result<AdvanceNotification, Status>>;
    async fn wait_for_advance(
        &self,
        request: Request<WaitForAdvanceRequest>,
    ) -> Result<Response<Self::WaitForAdvanceStream>, Status> {
        CoordinatorTrait::wait_for_advance(self.0.as_ref(), request).await
    }
}

struct CoordinatorService {
    expected_workers: usize,
    workers: Arc<RwLock<HashMap<WorkerId, (String, u64)>>>,
    all_ready: Arc<Notify>,
    current_superstep: Arc<RwLock<Superstep>>,
    barrier_reported: Arc<RwLock<HashMap<u64, HashMap<WorkerId, u64>>>>,
    advance_tx: watch::Sender<u64>,
    verbose: bool,
}

impl CoordinatorService {
    fn new(expected_workers: usize, verbose: bool) -> (Self, watch::Receiver<u64>) {
        let (advance_tx, advance_rx) = watch::channel(0u64);
        let svc = Self {
            expected_workers,
            workers: Arc::new(RwLock::new(HashMap::new())),
            all_ready: Arc::new(Notify::new()),
            current_superstep: Arc::new(RwLock::new(Superstep::new(0))),
            barrier_reported: Arc::new(RwLock::new(HashMap::new())),
            advance_tx,
            verbose,
        };
        (svc, advance_rx)
    }
}

#[tonic::async_trait]
impl CoordinatorTrait for CoordinatorService {
    async fn register_worker(
        &self,
        request: Request<RegisterWorkerRequest>,
    ) -> Result<Response<RegisterWorkerResponse>, Status> {
        let r = request.into_inner();
        let worker_id = r.worker_id as WorkerId;
        let count = {
            let mut w = self.workers.write().await;
            w.insert(worker_id, (r.address, r.vertex_count));
            w.len()
        };
        if count >= self.expected_workers {
            self.all_ready.notify_waiters();
        }
        Ok(Response::new(RegisterWorkerResponse {}))
    }

    async fn report_superstep_done(
        &self,
        request: Request<ReportSuperstepDoneRequest>,
    ) -> Result<Response<ReportSuperstepDoneResponse>, Status> {
        let r = request.into_inner();
        let worker_id = r.worker_id as WorkerId;
        let superstep = r.superstep;
        let messages_sent = r.messages_sent;

        let worker_count = self.expected_workers;
        let mut barrier = self.barrier_reported.write().await;
        barrier.entry(superstep).or_default().insert(worker_id, messages_sent);

        if barrier.get(&superstep).unwrap().len() >= worker_count {
            let all_halted = barrier
                .get(&superstep)
                .unwrap()
                .values()
                .all(|&msgs| msgs == 0);
            barrier.remove(&superstep);

            if all_halted {
                let terminated = u64::MAX;
                eprintln!("  [coordinator] all workers halted (0 msgs) → terminating");
                let _ = self.advance_tx.send(terminated);
                let mut cur = self.current_superstep.write().await;
                *cur = Superstep::new(terminated);
            } else {
                let next = superstep + 1;
                eprintln!("  [coordinator] advancing step {superstep} → {next}");
                let _ = self.advance_tx.send(next);
                let mut cur = self.current_superstep.write().await;
                *cur = Superstep::new(next);
            }
        } else if self.verbose {
            let reported = barrier.get(&superstep).unwrap().len();
            eprintln!("  [coordinator] worker {worker_id} reported step {superstep} ({reported}/{worker_count})");
        }

        Ok(Response::new(ReportSuperstepDoneResponse {}))
    }

    async fn wait_for_all_ready(
        &self,
        _request: Request<WaitForAllReadyRequest>,
    ) -> Result<Response<WaitForAllReadyResponse>, Status> {
        let count = self.workers.read().await.len();
        if count < self.expected_workers {
            self.all_ready.notified().await;
        }
        Ok(Response::new(WaitForAllReadyResponse {}))
    }

    async fn get_current_superstep(
        &self,
        _request: Request<GetCurrentSuperstepRequest>,
    ) -> Result<Response<GetCurrentSuperstepResponse>, Status> {
        let cur = self.current_superstep.read().await;
        Ok(Response::new(GetCurrentSuperstepResponse {
            superstep: cur.step,
        }))
    }

    type WaitForAdvanceStream = tokio_stream::wrappers::ReceiverStream<Result<AdvanceNotification, Status>>;

    async fn wait_for_advance(
        &self,
        request: Request<WaitForAdvanceRequest>,
    ) -> Result<Response<Self::WaitForAdvanceStream>, Status> {
        let from = request.into_inner().from_superstep;
        let mut advance_rx = self.advance_tx.subscribe();
        // Check current value first: if we subscribed after the advance already happened,
        // wait_for would block forever (it only wakes on *new* changes). Use borrow() to
        // detect the "already advanced" case.
        let step = if *advance_rx.borrow() > from {
            *advance_rx.borrow()
        } else {
            match advance_rx.wait_for(|step| *step > from).await {
                Ok(guard) => *guard,
                Err(_) => return Err(Status::unavailable("coordinator closed")),
            }
        };
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let _ = tx.send(Ok(AdvanceNotification { superstep: step })).await;
        drop(tx);
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
}

pub async fn run_coordinator_server(
    addr: std::net::SocketAddr,
    expected_workers: usize,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (svc, _) = CoordinatorService::new(expected_workers, verbose);
    let svc = Arc::new(svc);

    tonic::transport::Server::builder()
        .add_service(CoordinatorServer::new(ArcCoordinator(svc)))
        .serve(addr)
        .await?;
    Ok(())
}
