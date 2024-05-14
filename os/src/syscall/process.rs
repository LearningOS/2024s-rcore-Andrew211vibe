//! Process management syscalls
use core::mem::size_of;

use crate::{
    config::MAX_SYSCALL_NUM, mm::{translated_byte_buffer, PhysAddr, VirtAddr}, task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_task_info, ppn_by_vpn, suspend_current_and_run_next, task_mmap, task_munmap, TaskStatus
    }, timer::get_time_us
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
}

fn _va_to_pa(va: VirtAddr) -> Option<PhysAddr> {
    let offset = va.page_offset();
    let ppn = ppn_by_vpn(va.floor());
    match ppn {
        Some(ppn) => Some(PhysAddr::from((ppn.0 << 12) | offset)),
        _ => {
            error!("sys_va2pa() failed");
            None
        }
    }
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    // let ts = va_to_pa(VirtAddr::from(_ts as usize));
    // if let Some(pa) = ts {
    //     let us = get_time_us();
    //     let ts = pa.0 as *mut TimeVal;
    //     unsafe {
    //         *ts = TimeVal {
    //             sec: us / 1_000_000,
    //             usec: us % 1_000_000,
    //         };
    //     }
    //     0
    // } else {
    //     error!("sys_get_time() failed");
    //     -1
    // }
    let buffers = translated_byte_buffer(current_user_token(), _ts as *const u8, size_of::<TimeVal>());
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let mut time_val_ptr = &time_val as *const _ as *const u8;
    for buffer in buffers {
        unsafe {
            time_val_ptr.copy_to(buffer.as_mut_ptr(), buffer.len());
            time_val_ptr = time_val_ptr.add(buffer.len());
        }
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    if _ti.is_null() {
        return -1
    }
    // let ti = va_to_pa(VirtAddr::from(_ti as usize));
    // if let Some(pa) = ti {
    //     let ti = pa.0 as *mut TaskInfo;
    //     get_task_info(ti);
    //     0
    // } else {
    //     -1
    // }
    let ti = translated_byte_buffer(
        current_user_token(), 
        _ti as *const u8, 
        size_of::<TaskInfo>()
    )[0].as_ptr() as *mut TaskInfo;
    get_task_info(ti);
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    task_mmap(_start, _len, _port)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    task_munmap(_start, _len)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
