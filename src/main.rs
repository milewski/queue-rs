use std::sync::{Arc, Mutex};
use std::thread::{JoinHandle, sleep, spawn};
use std::time::{Duration, Instant};

struct Task {
    before_start: Box<dyn Fn() -> () + Send>,
    callable: Box<dyn Fn() -> () + Send>,
    on_complete: Box<dyn Fn() -> () + Send>,
}

impl Task {
    fn new(callable: Box<dyn Fn() -> () + Send>) -> Task {
        Task {
            callable,
            before_start: Box::new(|| {}),
            on_complete: Box::new(|| {}),
        }
    }
}

struct Queue {
    collection: Arc<Mutex<Vec<Task>>>,
}

struct QueueSetting {
    /// How many tasks can run every `interval_in_seconds`
    limit: usize,
    /// How thread to use to run task concurrently
    threads: usize,
    /// How many seconds to wait before resting the limit
    interval_in_seconds: u64,
}

struct QueueExecutor {
    collection: Arc<Mutex<Vec<Task>>>,
    setting: QueueSetting,
}

impl QueueExecutor {
    fn start(self) -> () {
        spawn(move || {
            let mut started_at = Instant::now();
            let interval = Duration::from_secs(self.setting.interval_in_seconds);
            let mut active_tasks_count = 0;

            loop {
                let elapsed = started_at.elapsed();

                if elapsed >= interval {
                    started_at = Instant::now();
                    active_tasks_count = 0;
                }

                if active_tasks_count >= self.setting.limit {
                    continue;
                }

                let mut collection = self.collection
                    .lock()
                    .expect("Unable to add task to the queue due inability to acquire lock.");

                let mut handles: Vec<JoinHandle<()>> = Vec::new();
                let tail: usize = match collection.len() {
                    length if length > self.setting.limit && length < self.setting.threads => self.setting.limit,
                    length if length > self.setting.threads => match length {
                        _ if self.setting.threads < self.setting.limit => self.setting.threads,
                        _ => self.setting.limit
                    },
                    length if length == 0 => continue,
                    length => length,
                };

                let callbacks: Vec<Task> = collection
                    .drain(0..tail)
                    .collect();

                for task in callbacks {
                    active_tasks_count += 1;
                    handles.push(spawn(move || {
                        (task.before_start)();
                        (task.callable)();
                        (task.on_complete)();
                    }));
                }

                for handle in handles {
                    handle.join().unwrap();
                }
            }
        });
    }
}

impl Queue {
    fn new(setting: QueueSetting) -> Queue {
        let collection = Arc::new(Mutex::new(Vec::new()));
        let clone = Arc::clone(&collection);

        let executor = QueueExecutor {
            collection,
            setting,
        };

        executor.start();

        Queue {
            collection: clone,
        }
    }
    fn add(&mut self, task: Task) {
        let mut collection = self.collection
            .lock()
            .expect("Unable to add task to the queue due inability to acquire lock.");

        collection.push(task);
    }
}

fn main() {
    let mut queue = Queue::new(
        QueueSetting {
            limit: 5,
            threads: 5,
            interval_in_seconds: 1,
        }
    );

    for index in 1..1000 {
        queue.add(
            Task::new(
                Box::new(move || println!("Task: {}", index))
            ),
        );
    }

    sleep(Duration::from_secs(9999));
}
