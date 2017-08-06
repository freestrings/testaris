use tc;
use app;

use std::sync::Mutex;

lazy_static! {
    static ref MESSAGE: Mutex<Vec<tc::Msg>> = Mutex::new(vec![]);
    static ref EVENT_Q: Mutex<Vec<tc::BlockEvent>> = Mutex::new(vec![]);
}

pub trait OpEvent {
    fn create(&mut self, worker_count: u8);
    fn init(&mut self, tetris_per_worker: u32);
    fn trigger_block_event(&mut self, event: tc::BlockEvent);
    fn send_app_event(&mut self, event: tc::AppEvent);
    fn received(&mut self) -> Vec<tc::Msg>;
}

pub struct EventMgr<T> {
    worker_handles: Vec<T>,
}

#[cfg(target_arch = "wasm32")]
mod wasm32 {
    use super::*;

    use tc;
    use asm;
    use serde_json;
    use libc;

    use std::os::raw::{c_char, c_int, c_void};
    use std::ptr;
    use std::ffi::CString;
    use std::mem;

    extern "C" fn em_worker_callback_func(data: *mut c_char, size: c_int, _user_args: *mut c_void) {
        let raw_msg: &[u8] =
            unsafe { mem::transmute(::std::slice::from_raw_parts(data, size as usize - 1)) };

        let msg = String::from_utf8(raw_msg.to_vec()).unwrap();
        let msg = serde_json::from_str::<tc::Msg>(msg.as_str()).unwrap();

        MESSAGE.lock().unwrap().push(msg);
    }

    extern "C" fn main_loop_callback(arg: *mut c_void) {
        unsafe {
            let mut app: &mut app::App = mem::transmute(arg);
            app.run();
        }
    }

    impl EventMgr<c_int> {
        pub fn new() -> EventMgr<c_int> {
            EventMgr { worker_handles: Vec::new() }
        }
    }

    impl OpEvent for EventMgr<c_int> {
        fn create(&mut self, worker_count: u8) {
            let mut worker_handles: Vec<c_int> = (0..worker_count)
                .map(|_| {
                    let resource = CString::new("tetrisworker.js").unwrap();
                    unsafe { asm::emscripten_create_worker(resource.as_ptr()) }
                })
                .collect();

            self.worker_handles.append(&mut worker_handles);
        }

        fn init(&mut self, tetris_per_worker: u32) {
            for worker_index in 0..self.worker_handles.len() {
                self.send_app_event(tc::AppEvent::InitWorker(
                    worker_index as u8,
                    tetris_per_worker,
                ));
            }

            for worker_index in 0..self.worker_handles.len() as u8 {
                for tetris_index in 0..tetris_per_worker {
                    self.send_app_event(tc::AppEvent::InitTetris(worker_index, tetris_index));
                }
            }
        }

        fn trigger_block_event(&mut self, event: tc::BlockEvent) {
            EVENT_Q.lock().unwrap().push(event);
        }

        fn send_app_event(&mut self, event: tc::AppEvent) {
            let json = serde_json::to_string(&event).expect("[main] Serialize error");
            let send = CString::new(json).unwrap();
            let send = send.into_raw();
            let len = unsafe { libc::strlen(send) as i32 };
            let method = CString::new("on").unwrap();

            unsafe {
                asm::emscripten_call_worker(
                    self.worker_handles[event.worker_id() as usize],
                    method.as_ptr(),
                    send,
                    len,
                    Some(em_worker_callback_func),
                    ptr::null_mut(),
                );
            }
        }

        fn received(&mut self) -> Vec<tc::Msg> {
            let mut messages = MESSAGE.lock().unwrap();
            let messages = messages.drain(..).collect();
            messages
        }
    }

    pub fn event_loop(mut app: Box<app::App>) {
        let app_ptr = &mut *app as *mut app::App as *mut c_void;
        unsafe {
            asm::emscripten_set_main_loop_arg(Some(main_loop_callback), app_ptr, 0, 1);
        }
        mem::forget(app);
    }

}

pub fn event_loop(app: Box<app::App>) {
    if cfg!(target_arch = "wasm32") {
        wasm32::event_loop(app);
    }
}
