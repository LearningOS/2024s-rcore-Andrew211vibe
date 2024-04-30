use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task, TaskControlBlock};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use super::sys_gettid;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}

fn set_need_resource(flag: bool, id: usize) {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    assert!(inner.need.replace((flag, id)).is_none());
}

fn mark_resource_released(flag: bool, id: usize) {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if let Some(pos) = inner
        .allocation
        .iter()
        .position(|res| res.0 == flag && res.1 == id)
    {
        inner.allocation.swap_remove(pos);
    }
}

/// deadlock detectiong algorithm
fn deadlock_detect(
    tasks: &Vec<Option<Arc<TaskControlBlock>>>,
    mutex_list: &Vec<Option<Arc<dyn Mutex>>>,
    semaphore_list: &Vec<Option<Arc<Semaphore>>>,
    flag: bool,
    id: usize,
) -> bool {
    let len = mutex_list.len()  + semaphore_list.len();

    let mut allocation = Vec::<u32>::new();
    let mut need = Vec::<u32>::new();
    allocation.resize(len * tasks.len(), 0);
    need.resize(len * tasks.len(), 0);

    let mut work = Vec::<u32>::new();
    work.resize(len, 0);

    let fetch_id = |flag, id| -> usize {
        match flag {
            true => id + mutex_list.len(),
            false => id,
        }
    };
    for (tid, tcb) in tasks.iter().enumerate() {
        if let Some(task) = tcb {
            let inner = task.inner_exclusive_access();
            for allc in &inner.allocation {
                allocation[fetch_id(allc.0, allc.1) + len * tid] += 1;
            }
            if let Some(n) = &inner.need {
                need[fetch_id(n.0, n.1) + len * tid] += 1;
            }
        }
    }
    need[fetch_id(flag, id) + len * sys_gettid() as usize] += 1;

    for (i, option) in mutex_list.iter().enumerate() {
        if let Some(mutex) = option {
            if !mutex.is_locked() {
                work[i] += 1;
            }
        }
    }
    for (i, option) in semaphore_list.iter().enumerate() {
        if let Some(semaphore) = option {
            let count = semaphore.inner.exclusive_access().count;
            if count > 0 {
                work[i + mutex_list.len()] += count as u32;
            }
        }
    }

    // default finish
    let mut finish = vec![false; tasks.len()];
    loop {
        let task = finish.iter().enumerate().find(|(tid, finished)| {
            if **finished {
                false
            } else {
                for j in 0..len {
                    if need[tid * len + j] > work[j] {
                        return false;
                    }
                }
                true
            }
        });
        if let Some((tid, _)) = task {
            finish[tid] = true;
            for j in 0..len {
                work[j] += allocation[tid * len + j];
            }
        } else {
            break;
        }
    }
    finish.contains(&false)
}

/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    if process_inner.dead_lock_detect
        && deadlock_detect(
            &process_inner.tasks,
            &process_inner.mutex_list,
            &process_inner.semaphore_list,
            false,
            mutex_id,
        )
    {
        return -0xDEAD;
    }
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    set_need_resource(false, mutex_id);
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mark_resource_released(false, mutex_id);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    mark_resource_released(true, sem_id);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    if process_inner.dead_lock_detect
        && deadlock_detect(
            &process_inner.tasks,
            &process_inner.mutex_list,
            &process_inner.semaphore_list,
            true,
            sem_id,
        )
    {
        return -0xDEAD;
    }
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    set_need_resource(true, sem_id);
    sem.down();
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    mark_resource_released(false, mutex_id);
    set_need_resource(false, mutex_id);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect NOT IMPLEMENTED");
    let proc = current_process();
    let mut proc_inner = proc.inner_exclusive_access();
    if _enabled == 1 || _enabled == 0 {
        proc_inner.dead_lock_detect = _enabled != 0;
        0
    } else {
        error!("sys_enable_deadlock_detect() failed");
        -1
    }
}
