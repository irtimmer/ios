use alloc::vec;
use alloc::vec::Vec;
use alloc::sync::Arc;

use lock_api::MappedRwLockWriteGuard;

use spin::{Mutex, RwLock as RawRwLock};
use spin::lock_api::{RwLockWriteGuard, RwLock};

use crate::arch::system::ThreadContext;
use crate::process::{Thread, Process};

pub struct Scheduler {
    pub threads: RwLock<Vec<Thread>>,
    cur_thread: Mutex<usize>,
}

impl Scheduler {
    pub fn new() -> Self {
        let kernel_process = Arc::new(RawRwLock::new(Process::empty()));
        let kernel_thread = Thread::new_current(kernel_process);
        Self {
            threads: RwLock::new(vec![kernel_thread]),
            cur_thread: Mutex::new(0)
        }
    }

    pub fn get_current_context<'a>(&self) -> MappedRwLockWriteGuard<'_, RawRwLock<()>, Thread> {
        let cur_thread = self.cur_thread.lock();
        RwLockWriteGuard::map(self.threads.write(), |f: &mut Vec<Thread>| &mut f[*cur_thread])
    }

    pub unsafe fn run_next(&self) -> ! {
        let tasks_len = self.threads.read().len();
        let state = {
            let mut cur_task = self.cur_thread.lock();
            *cur_task = (*cur_task + 1) % tasks_len;
            let thread = &self.threads.read()[*cur_task];
            thread.activate()
        };
        state.activate();
    }
}
