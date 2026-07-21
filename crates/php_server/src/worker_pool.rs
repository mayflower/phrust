//! Dedicated PHP worker threads.
//!
//! A fixed set of pinned OS threads own
//! PHP execution for the lifetime of the process. Jobs and finished
//! responses are `Send` (see `worker_payload_tests` in `php_request`);
//! execution-side `Rc` state never crosses a thread boundary because the
//! entire synchronous request core runs inside the worker.

use std::sync::mpsc;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::{
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
};

use tokio::sync::oneshot;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const DEFAULT_PHP_WORKER_STACK_BYTES: usize = 16 * 1024 * 1024;
const _: () = assert!(DEFAULT_PHP_WORKER_STACK_BYTES <= 16 * 1024 * 1024);

/// Worker stack size for native spills and runtime helpers. PHP call depth is
/// bounded independently by the VM, so pinned workers do not reserve the
/// historical 128 MiB Tokio stack. The old variable remains a compatibility
/// fallback for deployments that configured both pools together.
fn php_worker_stack_bytes() -> usize {
    std::env::var("PHRUST_SERVER_PHP_WORKER_STACK_BYTES")
        .or_else(|_| std::env::var("PHRUST_SERVER_TOKIO_WORKER_STACK_BYTES"))
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_PHP_WORKER_STACK_BYTES)
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_PHP_WORKER_STACK_BYTES, PhpWorkerPool, WorkerPoolError};
    use std::sync::{Arc, Barrier};

    #[test]
    fn default_php_worker_stack_is_bounded() {
        assert!(std::hint::black_box(DEFAULT_PHP_WORKER_STACK_BYTES) <= 16 * 1024 * 1024);
    }

    #[tokio::test]
    async fn serial_requests_reuse_the_warm_worker() {
        let pool = PhpWorkerPool::new(4);
        let first = pool.execute(current_worker_name).await.unwrap();
        let second = pool.execute(current_worker_name).await.unwrap();
        let third = pool.execute(current_worker_name).await.unwrap();

        assert_eq!(first, "php-worker-0");
        assert_eq!(second, first);
        assert_eq!(third, first);
    }

    #[tokio::test]
    async fn concurrent_requests_still_use_distinct_workers() {
        let pool = PhpWorkerPool::new(2);
        let barrier = Arc::new(Barrier::new(2));
        let left_barrier = Arc::clone(&barrier);
        let right_barrier = Arc::clone(&barrier);
        let (left, right) = tokio::join!(
            pool.execute(move || {
                left_barrier.wait();
                current_worker_name()
            }),
            pool.execute(move || {
                right_barrier.wait();
                current_worker_name()
            })
        );

        assert_ne!(left.unwrap(), right.unwrap());
    }

    #[tokio::test]
    async fn panicking_job_returns_error_and_retires_worker_capacity() {
        let pool = PhpWorkerPool::new(1);

        let error = pool
            .execute(|| -> () { panic!("simulated PHP worker failure") })
            .await
            .expect_err("panicking worker job must fail");
        assert_eq!(error, WorkerPoolError::WorkerPanicked);

        let error = pool
            .execute(|| 42)
            .await
            .expect_err("retired pool must reject later work");
        assert_eq!(error, WorkerPoolError::Closed);
    }

    fn current_worker_name() -> String {
        std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_owned()
    }
}

/// Type-erased worker computation and its post-release completion callback.
///
/// The callback publishes the result only after the worker has returned to
/// the idle set. A caller that immediately submits the next serial request
/// therefore sees the just-warmed worker and does not spread one-request
/// allocator high-water across the entire pool.
type CompletionCallback = Box<dyn FnOnce() + Send + 'static>;

struct Completion {
    callback: CompletionCallback,
    worker_reusable: bool,
}

type Job = Box<dyn FnOnce() -> Completion + Send + 'static>;

struct WorkerJob {
    task: Job,
    worker: usize,
    idle_workers: std::sync::Arc<Mutex<Vec<usize>>>,
    permit: OwnedSemaphorePermit,
    available_workers: Arc<Semaphore>,
    healthy_workers: Arc<AtomicUsize>,
}

/// Fixed pool of dedicated PHP worker threads.
#[derive(Debug)]
pub(crate) struct PhpWorkerPool {
    workers: Vec<mpsc::Sender<WorkerJob>>,
    idle_workers: std::sync::Arc<Mutex<Vec<usize>>>,
    available_workers: std::sync::Arc<Semaphore>,
    healthy_workers: Arc<AtomicUsize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkerPoolError {
    Closed,
    NoWorker,
    WorkerUnavailable,
    WorkerPanicked,
    ReplyDropped,
}

impl fmt::Display for WorkerPoolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Closed => "PHP worker pool is closed",
            Self::NoWorker => "PHP worker pool has no idle worker",
            Self::WorkerUnavailable => "PHP worker is unavailable",
            Self::WorkerPanicked => "PHP worker job panicked",
            Self::ReplyDropped => "PHP worker reply was dropped",
        })
    }
}

impl PhpWorkerPool {
    /// Spawns `workers` dedicated PHP threads sharing one job queue.
    pub(crate) fn new(workers: usize) -> Self {
        let worker_count = workers.max(1);
        let idle_workers =
            std::sync::Arc::new(Mutex::new((0..worker_count).rev().collect::<Vec<_>>()));
        let available_workers = std::sync::Arc::new(Semaphore::new(worker_count));
        let healthy_workers = Arc::new(AtomicUsize::new(worker_count));
        let mut senders = Vec::with_capacity(worker_count);
        for index in 0..worker_count {
            let (sender, receiver) = mpsc::channel::<WorkerJob>();
            senders.push(sender);
            std::thread::Builder::new()
                .name(format!("php-worker-{index}"))
                .stack_size(php_worker_stack_bytes())
                .spawn(move || {
                    while let Ok(job) = receiver.recv() {
                        let WorkerJob {
                            task,
                            worker,
                            idle_workers,
                            permit,
                            available_workers,
                            healthy_workers,
                        } = job;
                        let completion = task();
                        if completion.worker_reusable {
                            if let Ok(mut idle) = idle_workers.lock() {
                                idle.push(worker);
                            }
                            drop(permit);
                        } else {
                            // A panicking PHP job may have corrupted
                            // thread-local engine state. Retire this worker's
                            // capacity permanently instead of advertising it
                            // as healthy again.
                            permit.forget();
                            if healthy_workers.fetch_sub(1, Ordering::AcqRel) == 1 {
                                available_workers.close();
                            }
                        }
                        (completion.callback)();
                    }
                })
                .expect("spawn php worker thread");
        }
        Self {
            workers: senders,
            idle_workers,
            available_workers,
            healthy_workers,
        }
    }

    /// Submits a synchronous job to a pinned worker. Pool defects are explicit;
    /// PHP is never executed on the caller's Tokio transport thread.
    pub(crate) async fn submit<T, F>(
        &self,
        job: F,
    ) -> Result<oneshot::Receiver<Result<T, WorkerPoolError>>, WorkerPoolError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let (reply_sender, reply_receiver) = oneshot::channel();
        let task: Job = Box::new(move || {
            let result =
                catch_unwind(AssertUnwindSafe(job)).map_err(|_| WorkerPoolError::WorkerPanicked);
            let worker_reusable = result.is_ok();
            Completion {
                callback: Box::new(move || {
                    let _ = reply_sender.send(result);
                }),
                worker_reusable,
            }
        });
        let permit = match std::sync::Arc::clone(&self.available_workers)
            .acquire_owned()
            .await
        {
            Ok(permit) => permit,
            Err(_) => return Err(WorkerPoolError::Closed),
        };
        let worker = self
            .idle_workers
            .lock()
            .ok()
            .and_then(|mut idle| idle.pop());
        let Some(worker) = worker else {
            drop(permit);
            return Err(WorkerPoolError::NoWorker);
        };
        let queued = WorkerJob {
            task,
            worker,
            idle_workers: std::sync::Arc::clone(&self.idle_workers),
            permit,
            available_workers: Arc::clone(&self.available_workers),
            healthy_workers: Arc::clone(&self.healthy_workers),
        };
        match self.workers[worker].send(queued) {
            Ok(()) => Ok(reply_receiver),
            Err(mpsc::SendError(failed)) => {
                // This sender is permanently unusable. Do not return its
                // availability permit to the dispatcher.
                failed.permit.forget();
                if failed.healthy_workers.fetch_sub(1, Ordering::AcqRel) == 1 {
                    failed.available_workers.close();
                }
                drop(failed.task);
                Err(WorkerPoolError::WorkerUnavailable)
            }
        }
    }

    pub(crate) async fn execute<T, F>(&self, job: F) -> Result<T, WorkerPoolError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.submit(job)
            .await?
            .await
            .map_err(|_| WorkerPoolError::ReplyDropped)?
    }
}
