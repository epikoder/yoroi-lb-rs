use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use log::{debug, error};

pub static QUEUE: once_cell::sync::Lazy<Queue> = once_cell::sync::Lazy::new(|| Queue::new());

pub struct Job {
    pub title: String,
    pub handler: Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'static>>,
}

pub struct Queue {
    sender: Sender<Option<Job>>,
    receiver: Mutex<Receiver<Option<Job>>>,
}

impl Queue {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Queue {
            sender,
            receiver: Mutex::new(receiver),
        }
    }

    pub fn push(&self, job: Job) {
        self.sender.send(Some(job)).unwrap();
    }

    pub fn pop(&self) -> Option<Job> {
        self.receiver.lock().unwrap().recv().unwrap()
    }

    pub fn end(&self) {
        self.sender.send(None).unwrap();
    }
}

pub fn dispatch(job: Job) {
    tokio::spawn(async {
        QUEUE.push(job);
    });
}

pub async fn run_block(n: Option<u64>) {
    let n = Arc::new(Mutex::new(n));
    if let Some(ref n) = *n.lock().unwrap() {
        debug!("Queue: max allowed executions :{n}");
    }

    loop {
        if let Some(job) = QUEUE.pop() {
            let n_clone = Arc::clone(&n);
            tokio::spawn(async move {
                match job.handler.await {
                    Ok(_) => debug!("Queue: job: {} completed", job.title),
                    Err(e) => error!("Queue: job: {} returned error: {}", job.title, e),
                }
                let mut data = n_clone.lock().unwrap();
                if let Some(ref mut value) = *data {
                    *value -= 1;
                    if *value == 0 {
                        debug!("task limit reached, signaling to terminate queue");
                        shutdown();
                    }
                    debug!("task completed, remaining: {}", value);
                }
            });
        } else {
            break;
        }
    }
}

pub fn shutdown() {
    debug!("Queue: graceful shutdown...");
    QUEUE.end();
}

#[cfg(feature = "test")]
pub fn start_queue(n: Option<u64>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { run_block(n).await });
    })
}

#[cfg(test)]
mod tests {
    use log::info;
    use tokio::time::sleep;

    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_queue() {
        env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .init();
        let j = 100_u64;
        let job = std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { run_block(Some(j)).await });
        });

        sleep(Duration::from_secs(2)).await;
        let n = Arc::new(Mutex::new(0));
        for i in 0..j {
            let n_clone = Arc::clone(&n);
            dispatch(Job {
                title: "Test".to_string(),
                handler: Box::pin(async move {
                    *n_clone.lock().unwrap() += 1;
                    debug!("Completed task {i}");
                    Ok(())
                }),
            });
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let _ = job.join();
        info!("Tasks .... {:?}", n.lock().unwrap());
    }
}
