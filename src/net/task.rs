use std::time::{Duration, Instant};

use super::{Socket, error::Result};

/// A type alias for a callback function that takes a mutable reference to a `Socket` and returns a `Result<(), NetError>`.
pub type TaskCallback = Box<dyn FnMut(&mut Socket) -> Result<()> + Send + Sync>;

/// Represents a task that can be scheduled to run at a specific frequency.
#[allow(dead_code)]
pub(crate) struct Task {
    id: usize,              // ID for the task.
    name: String,           // Name of the task.
    frequency_ms: u64,      // Frequency of the task in milliseconds.
    next_run: Instant,      // Next run time of the task.
    callback: TaskCallback, // Callback function to execute when the task runs.
}

impl Task {
    /// Creates a new task with the given frequency and callback function.
    pub fn new<F, N: Into<String>>(id: usize, name: N, frequency_ms: u64, callback: F) -> Self
    where
        F: FnMut(&mut Socket) -> Result<()> + Send + Sync + 'static,
    {
        Self {
            id,
            name: name.into(),
            frequency_ms,
            next_run: Instant::now() + Duration::from_millis(frequency_ms),
            callback: Box::new(callback),
        }
    }

    /// Checks if the task is ready to run based on the current time.
    #[inline]
    pub fn is_ready(&self) -> bool {
        Instant::now() >= self.next_run
    }

    /// Run the callback assigned to the task.
    #[inline]
    pub fn run(&mut self, socket: &mut Socket) -> Result<()> {
        (self.callback)(socket)
    }

    /// Resets the task's next run time to the current time plus the frequency.
    #[inline]
    pub fn reset(&mut self) {
        self.next_run = Instant::now() + Duration::from_millis(self.frequency_ms);
    }
}

/// Represents a task scheduler that manages multiple tasks.
pub(crate) struct TaskScheduler {
    frequency_ms: u64, // Frequency of running the scheduler in milliseconds.
    next_run: Instant, // Next run time for the scheduler.
    tasks: Vec<Task>,  // List of tasks to be scheduled.
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl TaskScheduler {
    pub fn new(frequency_ms: u64) -> Self {
        Self {
            frequency_ms,
            next_run: Instant::now() + Duration::from_millis(frequency_ms),
            tasks: Vec::new(),
        }
    }

    /// Adds a new task to the scheduler.
    pub fn register<F, N: Into<String>>(&mut self, name: N, freq_ms: u64, callback: F) -> usize
    where
        F: Fn(&mut Socket) -> Result<()> + Send + Sync + 'static,
    {
        let task_id = self.tasks.len() + 1;
        self.tasks.push(Task::new(task_id, name, freq_ms, callback));
        self.sort();
        task_id
    }

    /// Sorts the tasks based on their next run time.
    pub fn sort(&mut self) {
        self.tasks.sort_by(|a, b| a.next_run.cmp(&b.next_run));
    }

    /// Checks if the scheduler is ready to be ran.
    pub fn is_ready(&self) -> bool {
        Instant::now() >= self.next_run
    }

    /// Executes the tasks that are ready to run.
    pub fn run(&mut self, socket: &mut Socket) -> Result<()> {
        let mut exec = false;

        for task in &mut self.tasks {
            if task.is_ready() {
                task.run(socket)?;
                task.reset();
                exec = true;
            } else {
                break;
            }
        }

        if exec {
            self.sort(); // Re-sort the tasks after execution.
        }

        // Update the next run time for the scheduler itself.
        self.next_run = Instant::now() + Duration::from_millis(self.frequency_ms);
        Ok(())
    }
}
