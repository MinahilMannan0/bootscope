//! Cooperative multitasking.
//!
//! A "task" is a small piece of work that runs in steps. The scheduler keeps a
//! list of tasks and, on every pass of the kernel's main loop, gives each task
//! one turn (round-robin). Each step must finish quickly and return, so the
//! other tasks and the keyboard get their turn too.
//!
//! This is COOPERATIVE multitasking (like early Windows and Mac OS): a task is
//! never interrupted. Preemptive multitasking would need timer interrupts and
//! context switching, and real processes would also need separate memory and
//! user mode - those are not implemented.

use crate::console::Console;
use crate::statusbar;
use crate::sysinfo::SystemInfo;
use crate::rtc;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// What a task may use during its turn.
pub struct Context<'a, 'b> {
    pub console: &'a mut Console<'b>,
    pub info: &'a SystemInfo,
    pub task_count: usize,
}

pub trait Task {
    fn name(&self) -> &'static str;
    /// Do one small piece of work. Return false when the task is finished.
    fn step(&mut self, ctx: &mut Context<'_, '_>) -> bool;
    /// A short description for the `ps` command.
    fn status(&self) -> String;
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

    pub fn summaries(&self) -> Vec<TaskSummary> {
        self.tasks
            .iter()
            .map(|e| TaskSummary { pid: e.pid, name: e.task.name(), runs: e.runs, status: e.task.status() })
            .collect()
    }

    /// Give every task one turn (round-robin), then drop finished tasks.
    pub fn run_once(&mut self, console: &mut Console<'_>, info: &SystemInfo) {
        let task_count = self.tasks.len();
        let mut ctx = Context { console, info, task_count };
        for entry in self.tasks.iter_mut() {
            entry.runs += 1;
            entry.alive = entry.task.step(&mut ctx);
        }
        self.tasks.retain(|e| e.alive);
    }
}

/// How many turns a time-checking task waits between clock reads.
/// Reading the clock chip is slow, so we do not do it on every turn.
const CHECK_EVERY: u32 = 500;

/// Updates the status bar once per second.
pub struct ClockTask {
    turns: u32,
    last_second: u8,
}

impl ClockTask {
    pub fn new() -> Self {
        ClockTask { turns: 0, last_second: 99 }
    }
}

impl Task for ClockTask {
    fn name(&self) -> &'static str {
        "clock"
    }

    fn step(&mut self, ctx: &mut Context<'_, '_>) -> bool {
        self.turns = self.turns.wrapping_add(1);
        if self.turns % CHECK_EVERY == 0 {
            let now = rtc::now();
            if now.second != self.last_second {
                self.last_second = now.second;
                statusbar::draw_live(ctx.console, ctx.info, now, ctx.task_count);
            }
        }
        true
    }

    fn status(&self) -> String {
        String::from("updates the status bar every second")
    }
}

/// Blinks the text cursor once per second.
pub struct CursorTask {
    turns: u32,
    last_second: u8,
    visible: bool,
}

impl CursorTask {
    pub fn new() -> Self {
        CursorTask { turns: 0, last_second: 99, visible: false }
    }
}

impl Task for CursorTask {
    fn name(&self) -> &'static str {
        "cursor"
    }

    fn step(&mut self, ctx: &mut Context<'_, '_>) -> bool {
        self.turns = self.turns.wrapping_add(1);
        if self.turns % CHECK_EVERY == 0 {
            let second = rtc::now().second;
            if second != self.last_second {
                self.last_second = second;
                self.visible = !self.visible;
                ctx.console.set_cursor(self.visible);
            }
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
