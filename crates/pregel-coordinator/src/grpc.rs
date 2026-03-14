//! gRPC server for the coordinator.

pub(crate) mod coordinator_proto {
    tonic::include_proto!("pregel.coordinator");
}

use coordinator_proto::coordinator_server::{Coordinator as CoordinatorTrait, CoordinatorServer};
use coordinator_proto::{
    AdvanceNotification, GetCurrentSuperstepRequest, GetCurrentSuperstepResponse,
    JobStartNotification, RegisterWorkerRequest, RegisterWorkerResponse,
    ReportJobResultsRequest, ReportJobResultsResponse, ReportSuperstepDoneRequest,
    ReportSuperstepDoneResponse, WaitForAdvanceRequest,
    WaitForAllReadyRequest, WaitForAllReadyResponse, WaitForJobStartRequest,
};
use pregel_common::{VertexId, WorkerId};
use pregel_core::{Algorithm, AlgoMetadata, PostFunction, Superstep};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, oneshot, watch, Notify, RwLock};
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
    type WaitForJobStartStream = tokio_stream::wrappers::ReceiverStream<Result<JobStartNotification, Status>>;
    async fn wait_for_job_start(
        &self,
        request: Request<WaitForJobStartRequest>,
    ) -> Result<Response<Self::WaitForJobStartStream>, Status> {
        CoordinatorTrait::wait_for_job_start(self.0.as_ref(), request).await
    }
    async fn report_job_results(
        &self,
        request: Request<ReportJobResultsRequest>,
    ) -> Result<Response<ReportJobResultsResponse>, Status> {
        self.0.report_job_results(request).await
    }
}

#[derive(Clone)]
pub struct JobParams {
    pub job_id: u32,
    pub algo: String,
    pub program: String,
    pub total_vertices: u64,
}

struct JobResultState {
    algo: String,
    collected: HashMap<WorkerId, Vec<(VertexId, Vec<u8>)>>,
    result_tx: Option<oneshot::Sender<serde_json::Value>>,
}

struct CoordinatorService {
    expected_workers: usize,
    workers: Arc<RwLock<HashMap<WorkerId, (String, u64)>>>,
    worker_last_seen: Arc<RwLock<HashMap<WorkerId, Instant>>>,
    all_ready: Arc<Notify>,
    current_superstep: Arc<RwLock<Superstep>>,
    barrier_reported: Arc<RwLock<HashMap<u64, HashMap<WorkerId, u64>>>>,
    advance_tx: watch::Sender<u64>,
    job_start_tx: broadcast::Sender<JobParams>,
    next_job_id: Arc<RwLock<u32>>,
    job_results: Arc<RwLock<HashMap<u32, JobResultState>>>,
    verbose: bool,
}

impl CoordinatorService {
    fn new(expected_workers: usize, verbose: bool) -> (Self, watch::Receiver<u64>, broadcast::Sender<JobParams>) {
        let (advance_tx, advance_rx) = watch::channel(0u64);
        let (job_start_tx, _) = broadcast::channel(16);
        let svc = Self {
            expected_workers,
            workers: Arc::new(RwLock::new(HashMap::new())),
            worker_last_seen: Arc::new(RwLock::new(HashMap::new())),
            all_ready: Arc::new(Notify::new()),
            current_superstep: Arc::new(RwLock::new(Superstep::new(0))),
            barrier_reported: Arc::new(RwLock::new(HashMap::new())),
            advance_tx,
            job_start_tx: job_start_tx.clone(),
            next_job_id: Arc::new(RwLock::new(1)),
            job_results: Arc::new(RwLock::new(HashMap::new())),
            verbose,
        };
        (svc, advance_rx, job_start_tx)
    }

    pub async fn submit_job(&self, algo: String, program: String) -> (JobParams, oneshot::Receiver<serde_json::Value>) {
        let total_vertices: u64 = self.workers.read().await.values().map(|(_, c)| c).sum();
        let job_id = {
            let mut id = self.next_job_id.write().await;
            let j = *id;
            *id = id.saturating_add(1);
            j
        };
        let (result_tx, result_rx) = oneshot::channel();
        let params = JobParams {
            job_id,
            algo: algo.clone(),
            program: program.clone(),
            total_vertices,
        };
        self.job_results.write().await.insert(
            job_id,
            JobResultState {
                algo: algo.clone(),
                collected: HashMap::new(),
                result_tx: Some(result_tx),
            },
        );
        let _ = self.job_start_tx.send(params.clone());
        *self.current_superstep.write().await = Superstep::new(0);
        self.advance_tx.send_replace(0);
        (params, result_rx)
    }

    fn apply_post_function(algo: &str, meta: &AlgoMetadata, mut combined: Vec<(VertexId, Vec<u8>)>) -> serde_json::Value {
        match &meta.post {
            PostFunction::ConcatAndSort | PostFunction::SingleValue => {
                combined.sort_by_key(|(id, _)| *id);
            }
            PostFunction::Concat => {}
        }
        let vertices_json: Vec<serde_json::Value> = combined
            .into_iter()
            .map(|(id, value)| {
                let value_json = decode_value_for_json(&value, meta);
                serde_json::json!({ "id": id, "value": value_json })
            })
            .collect();
        serde_json::json!({ "algo": algo, "vertices": vertices_json, "count": vertices_json.len() })
    }
}

fn decode_value_for_json(value: &[u8], _meta: &AlgoMetadata) -> serde_json::Value {
    if let Ok(v) = bincode::deserialize::<u64>(value) {
        return serde_json::json!(v);
    }
    if let Ok(v) = bincode::deserialize::<f64>(value) {
        return serde_json::json!(v);
    }
    serde_json::json!(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, value))
}

#[tonic::async_trait]
impl CoordinatorTrait for CoordinatorService {
    type WaitForJobStartStream = tokio_stream::wrappers::ReceiverStream<Result<JobStartNotification, Status>>;

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
        self.worker_last_seen.write().await.insert(worker_id, Instant::now());
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

        self.worker_last_seen.write().await.insert(worker_id, Instant::now());

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

            let max_steps = 200u64; // safety cap for PageRank etc
            let force_terminate = superstep >= max_steps;

            if all_halted || force_terminate {
                let terminated = u64::MAX;
                if force_terminate {
                    eprintln!("  [coordinator] max supersteps ({}) reached → forcing terminate", max_steps);
                } else {
                    eprintln!("  [coordinator] all workers halted (0 msgs) → terminating");
                }
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

    async fn wait_for_job_start(
        &self,
        _request: Request<WaitForJobStartRequest>,
    ) -> Result<Response<Self::WaitForJobStartStream>, Status> {
        let mut job_rx = self.job_start_tx.subscribe();
        let params = job_rx.recv().await.map_err(|e| {
            Status::unavailable(format!("job start channel closed: {}", e))
        })?;
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let _ = tx
            .send(Ok(JobStartNotification {
                job_id: params.job_id,
                algo: params.algo,
                program: params.program,
                total_vertices: params.total_vertices,
            }))
            .await;
        drop(tx);
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn report_job_results(
        &self,
        request: Request<ReportJobResultsRequest>,
    ) -> Result<Response<ReportJobResultsResponse>, Status> {
        let r = request.into_inner();
        let job_id = r.job_id;
        let worker_id = r.worker_id as WorkerId;
        let vertices: Vec<(VertexId, Vec<u8>)> = r
            .vertices
            .into_iter()
            .map(|v| (v.vertex_id, v.value))
            .collect();

        let expected = self.expected_workers;
        let mut job_map = self.job_results.write().await;
        if let Some(state) = job_map.get_mut(&job_id) {
            state.collected.insert(worker_id, vertices);
            if state.collected.len() >= expected {
                let algo_str = state.algo.clone();
                let mut combined = Vec::new();
                for parts in state.collected.values() {
                    combined.extend(parts.clone());
                }
                let algo = algo_str.parse::<Algorithm>().unwrap_or(Algorithm::ConnectedComponents);
                let meta = AlgoMetadata::for_algo(algo);
                let result = Self::apply_post_function(&algo_str, &meta, combined);
                if let Some(tx) = state.result_tx.take() {
                    let _ = tx.send(result);
                }
                job_map.remove(&job_id);
            }
        }
        Ok(Response::new(ReportJobResultsResponse {}))
    }
}

pub async fn run_coordinator_server(
    addr: std::net::SocketAddr,
    expected_workers: usize,
    verbose: bool,
    http_port: Option<u16>,
    worker_timeout_secs: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (svc, _, _) = CoordinatorService::new(expected_workers, verbose);
    let svc = Arc::new(svc);

    // Background task: detect workers that haven't reported within timeout, abort job
    let svc_timeout = Arc::clone(&svc);
    let timeout_secs = worker_timeout_secs;
    tokio::spawn(async move {
        let check_interval = Duration::from_secs(5);
        let timeout_duration = Duration::from_secs(timeout_secs);
        loop {
            tokio::time::sleep(check_interval).await;
            let barrier = svc_timeout.barrier_reported.read().await;
            // Find supersteps we're waiting on (incomplete barriers)
            let pending: Vec<(u64, HashSet<WorkerId>)> = barrier
                .iter()
                .filter(|(_, reported)| reported.len() < svc_timeout.expected_workers)
                .map(|(step, reported)| (*step, reported.keys().copied().collect()))
                .collect();
            drop(barrier);

            for (superstep, reported) in pending {
                let expected: HashSet<WorkerId> = (0..svc_timeout.expected_workers as WorkerId).collect();
                let missing: Vec<WorkerId> = expected.difference(&reported).copied().collect();
                if missing.is_empty() {
                    continue;
                }

                let last_seen = svc_timeout.worker_last_seen.read().await;
                let timed_out: Vec<WorkerId> = missing
                    .into_iter()
                    .filter(|&wid| {
                        last_seen
                            .get(&wid)
                            .map(|t| t.elapsed() > timeout_duration)
                            .unwrap_or(true) // never seen = consider timed out
                    })
                    .collect();
                drop(last_seen);

                if !timed_out.is_empty() {
                    eprintln!(
                        "  [coordinator] worker(s) {:?} timed out (no report for step {} in {}s) → aborting job",
                        timed_out, superstep, timeout_secs
                    );
                    let _ = svc_timeout.advance_tx.send(u64::MAX);
                    *svc_timeout.current_superstep.write().await = Superstep::new(u64::MAX);
                    svc_timeout.barrier_reported.write().await.remove(&superstep);
                    let mut job_map = svc_timeout.job_results.write().await;
                    let mut to_remove = None;
                    for (job_id, state) in job_map.iter_mut() {
                        if let Some(tx) = state.result_tx.take() {
                            let _ = tx.send(serde_json::json!({
                                "error": "worker_timeout",
                                "message": format!("Worker(s) {:?} timed out", timed_out),
                            }));
                            to_remove = Some(*job_id);
                            break;
                        }
                    }
                    if let Some(id) = to_remove {
                        job_map.remove(&id);
                    }
                    break;
                }
            }
        }
    });

    if let Some(port) = http_port {
        let http_addr: std::net::SocketAddr = format!("0.0.0.0:{}", port).parse()?;
        let svc_http = Arc::clone(&svc);
        tokio::spawn(async move {
            use axum::{extract::State, routing::post, Json, Router};
            #[derive(serde::Deserialize)]
            struct JobRequest {
                algo: String,
                #[serde(default)]
                program: String,
            }
            async fn submit_job_handler(
                State(s): State<Arc<CoordinatorService>>,
                Json(body): Json<JobRequest>,
            ) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
                let (params, result_rx) = s.submit_job(body.algo, body.program).await;
                let result = result_rx.await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                Ok(Json(serde_json::json!({
                    "job_id": params.job_id,
                    "algo": params.algo,
                    "result": result,
                })))
            }
            let app = Router::new()
                .route("/jobs", post(submit_job_handler))
                .with_state(svc_http);
            let listener = match tokio::net::TcpListener::bind(http_addr).await {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("coordinator HTTP bind failed: {}", e);
                    return;
                }
            };
            eprintln!("Coordinator HTTP API on http://{}/jobs (POST)", http_addr);
            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("coordinator HTTP server error: {}", e);
            }
        });
    }

    tonic::transport::Server::builder()
        .add_service(CoordinatorServer::new(ArcCoordinator(svc)))
        .serve(addr)
        .await?;
    Ok(())
}
