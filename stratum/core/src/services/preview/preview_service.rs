use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TrySendError, bounded};

use super::preview_worker::PreviewWorker;
use crate::structs::preview::{PreviewFrame, PreviewRequest, PreviewSource};

// Renders structure previews on its own thread, named `stratum-preview`, so a file read, a mesh build or a render
// never runs on the render thread. The UI submits a request when the view changes and takes the newest picture
// on a later frame.
// Bound: `QUEUE` requests wait; the thread takes the newest and drops the older ones. One finished picture waits;
// a newer one replaces it.
pub struct PreviewService {
    requests: Option<Sender<PreviewRequest>>,
    results: Receiver<PreviewFrame>,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl PreviewService {
    pub const QUEUE: usize = 4;
    const POLL: Duration = Duration::from_millis(100);

    pub fn new(source: PreviewSource) -> Self {
        let (request_tx, request_rx) = bounded::<PreviewRequest>(Self::QUEUE);
        let (result_tx, result_rx) = bounded::<PreviewFrame>(1);
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let stale = result_rx.clone();
        let spawned = std::thread::Builder::new().name("stratum-preview".to_string()).spawn(move || {
            Self::run(PreviewWorker::new(source), &request_rx, &result_tx, &stale, &thread_shutdown);
        });
        let thread = match spawned {
            Ok(handle) => Some(handle),
            Err(e) => {
                curia::error!("preview thread could not start", { error: e.to_string() });
                None
            }
        };
        Self { requests: Some(request_tx), results: result_rx, shutdown, thread }
    }

    pub fn new_shared(source: PreviewSource) -> Arc<Self> {
        Arc::new(Self::new(source))
    }

    // Never blocks. A full queue, or a thread that is gone, gives the request back.
    pub fn submit(&self, request: PreviewRequest) -> Result<(), PreviewRequest> {
        let Some(requests) = &self.requests else {
            return Err(request);
        };
        match requests.try_send(request) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(request) | TrySendError::Disconnected(request)) => Err(request),
        }
    }

    // The newest finished picture, if one arrived since the last call. Never blocks.
    pub fn take(&self) -> Option<PreviewFrame> {
        let mut newest = None;
        while let Ok(frame) = self.results.try_recv() {
            newest = Some(frame);
        }
        newest
    }

    fn run(
        mut worker: PreviewWorker,
        requests: &Receiver<PreviewRequest>,
        results: &Sender<PreviewFrame>,
        stale: &Receiver<PreviewFrame>,
        shutdown: &AtomicBool,
    ) {
        while !shutdown.load(Ordering::SeqCst) {
            let mut request = match requests.recv_timeout(Self::POLL) {
                Ok(request) => request,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => return,
            };
            while let Ok(newer) = requests.try_recv() {
                request = newer;
            }
            let mut frame = worker.handle(&request);
            loop {
                match results.try_send(frame) {
                    Ok(()) | Err(TrySendError::Disconnected(_)) => break,
                    Err(TrySendError::Full(back)) => {
                        // the waiting picture is older than this one
                        let _ = stale.try_recv();
                        frame = back;
                    }
                }
            }
        }
    }
}

impl Drop for PreviewService {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.requests = None;
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            curia::error!("preview thread ended with a panic", {});
        }
    }
}
