//! Cooperative multitasking.
//!
//! A "task" is a small piece of work that runs in steps. The scheduler keeps a
//! list of tasks and, on every pass of the kernel's main loop, gives each task
//! one turn (round-robin). Each step must finish quickly and return, so the
//! other tasks and the keyboard get their turn too.
//!
//! This is COOPERATIVE multitasking (like early Windows and Mac OS): a task is
//! never stopped in the middle of a step. Preemptive multitasking would also
//! need context switching (saving one task's registers and stack and loading
//! another's) from the timer interrupt, and real processes would need separate
//! memory and user mode - those are not implemented.
//!
//! When no task needs the CPU, the kernel sleeps with `hlt` until the next
//! interrupt (timer tick or key press), instead of spinning.

use crate::console::Console;
use crate::statusbar;
use crate::{interrupts, rtc};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// What a task may use during its turn.
pub struct Context<'a, 'b> {
    pub console: &'a mut Console<'b>,
    pub task_count: usize,
}

pub trait Task {
    fn name(&self) -> &'static str;
    /// Do one small piece of work. Return false when the task is finished.
    fn step(&mut self, ctx: &mut Context<'_, '_>) -> bool;
    /// A short description for the `ps` command.
    fn status(&self) -> String;
    /// True if the task wants to run continuously (then the kernel must not sleep).
    fn needs_cpu(&self) -> bool {
        false
    }
}

struct Entry {
    pid: u32,
    task: Box<dyn Task>,
    runs: u64,
    alive: bool,
}

pub struct TaskSummary {
    pub pid: u32,
    pub name: &'static str,
    pub runs: u64,
    pub status: String,
}

pub struct Scheduler {
    tasks: Vec<Entry>,
    next_pid: u32,
}

impl Scheduler {
    pub fn new() -> Self {
        // PID 0 = kernel main loop, PID 1 = shell; background tasks start at 2.
        Scheduler { tasks: Vec::new(), next_pid: 2 }
    }

    pub fn spawn(&mut self, task: Box<dyn Task>) -> u32 {
        let pid = self.next_pid;
        self.next_pid += 1;
        self.tasks.push(Entry { pid, task, runs: 0, alive: true });
        pid
    }

    /// Remove a task. Returns its name, or None if no such PID.
    pub fn kill(&mut self, pid: u32) -> Option<&'static str> {
        let index = self.tasks.iter().position(|e| e.pid == pid)?;
        Some(self.tasks.remove(index).task.name())
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Does any task want the CPU right now?
    pub fn needs_cpu(&self) -> bool {
        self.tasks.iter().any(|e| e.task.needs_cpu())
    }

    pub fn summaries(&self) -> Vec<TaskSummary> {
        self.tasks
            .iter()
            .map(|e| TaskSummary { pid: e.pid, name: e.task.name(), runs: e.runs, status: e.task.status() })
            .collect()
    }

    /// Give every task one turn (round-robin), then drop finished tasks.
    pub fn run_once(&mut self, console: &mut Console<'_>) {
        let task_count = self.tasks.len();
        let mut ctx = Context { console, task_count };
        for entry in self.tasks.iter_mut() {
            entry.runs += 1;
            entry.alive = entry.task.step(&mut ctx);
        }
        self.tasks.retain(|e| e.alive);
    }
}

/// Updates the status bar once per second (every 100 timer ticks).
pub struct ClockTask {
    last_second: u64,
}

impl ClockTask {
    pub fn new() -> Self {
        ClockTask { last_second: u64::MAX }
    }
}

impl Task for ClockTask {
    fn name(&self) -> &'static str {
        "clock"
    }

    fn step(&mut self, ctx: &mut Context<'_, '_>) -> bool {
        let second = interrupts::ticks() / interrupts::TIMER_HZ;
        if second != self.last_second {
            self.last_second = second;
            statusbar::draw_live(ctx.console, rtc::now(), second, ctx.task_count);
        }
        true
    }

    fn status(&self) -> String {
        String::from("updates the status bar every second")
    }
}

/// Blinks the text cursor twice per second.
pub struct CursorTask {
    last_half_second: u64,
    visible: bool,
}

impl CursorTask {
    pub fn new() -> Self {
        CursorTask { last_half_second: u64::MAX, visible: false }
    }
}

impl Task for CursorTask {
    fn name(&self) -> &'static str {
        "cursor"
    }

    fn step(&mut self, ctx: &mut Context<'_, '_>) -> bool {
        let half_second = interrupts::ticks() / (interrupts::TIMER_HZ / 2);
        if half_second != self.last_half_second {
            self.last_half_second = half_second;
            self.visible = !self.visible;
            ctx.console.set_cursor(self.visible);
        }
        true
    }

    fn status(&self) -> String {
        String::from(if self.visible { "blinking cursor (shown)" } else { "blinking cursor (hidden)" })
    }
}

/// CPU-heavy demo: finds prime numbers, testing one number per turn.
/// Shows that the shell stays responsive while work runs in the background.
pub struct PrimesTask {
    candidate: u64,
    found: u64,
    latest: u64,
}

const PRIMES_LIMIT: u64 = 2_000_000;

impl PrimesTask {
    pub fn new() -> Self {
        PrimesTask { candidate: 2, found: 0, latest: 0 }
    }
}

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

impl Task for PrimesTask {
    fn name(&self) -> &'static str {
        "primes"
    }

    fn step(&mut self, _ctx: &mut Context<'_, '_>) -> bool {
        if is_prime(self.candidate) {
            self.found += 1;
            self.latest = self.candidate;
        }
        self.candidate += 1;
        self.candidate <= PRIMES_LIMIT // finished when the limit is reached
    }

    fn status(&self) -> String {
        format!("{} primes found so far, latest {}", self.found, self.latest)
    }

    fn needs_cpu(&self) -> bool {
        true
    }
}

/// Create a task by name (for the `spawn` command).
pub fn create(name: &str) -> Option<Box<dyn Task>> {
    match name {
        "primes" => Some(Box::new(PrimesTask::new())),
        "clock" => Some(Box::new(ClockTask::new())),
        "cursor" => Some(Box::new(CursorTask::new())),
        _ => None,
    }
}
