#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate serde_json;
extern crate tetris_core;

#[cfg(target_arch = "wasm32")]
extern crate emscripten_sys as asm;

#[cfg(target_arch = "wasm32")]
pub use wasm32::on;

#[cfg(target_arch = "wasm32")]
mod wasm32 {
    use super::*;

    use std::ffi::CString;
    use std::mem;
    use std::os::raw::{c_char, c_int};
    use std::sync::Mutex;
    use std::slice;

    use tetris_core::*;

    lazy_static! {
        static ref TETRIS: Mutex<Vec<Tetris>> = Mutex::new(vec![]);
        static ref IDX: Mutex<Option<u8>> = Mutex::new(None);
    }

    #[allow(dead_code)]
    mod log {
        use super::asm;

        pub fn debug(mut msg: String) {
            msg.push('\0');
            unsafe {
                asm::emscripten_log(asm::EM_LOG_CONSOLE as i32, msg);
            }
        }

        pub fn error(mut msg: String) {
            msg.push('\0');
            unsafe {
                asm::emscripten_log(asm::EM_LOG_ERROR as i32, msg);
            }
        }
    }

    fn send_back(msg: Msg) {
        let json = serde_json::to_string(&msg).expect("[core] Serialze error\0");
        let send_back = CString::new(json).unwrap();
        let send_back = send_back.into_raw();
        let len = unsafe { libc::strlen(send_back) as i32 };

        unsafe {
            asm::emscripten_worker_respond(send_back, len + 1);
        }
    }

    fn into_raw<'a>(data: *mut c_char, size: c_int) -> &'a [u8] {
        unsafe { mem::transmute(slice::from_raw_parts(data, size as usize)) }
    }

    fn worker_guard(worker_id: u8) -> bool {
        match *IDX.lock().unwrap() {
            Some(ref idx) => worker_id.ne(idx),
            None => true,
        }
    }

    fn init_worker(worker_index: u8, tetris_count: u32) {
        if let Some(idx) = *IDX.lock().unwrap() {
            log::error(format!("already initialized: {}", idx));
            return;
        }

        *IDX.lock().unwrap() = Some(worker_index);

        for _ in 0..tetris_count {
            TETRIS.lock().unwrap().push(Tetris::new());
        }

        let event = AppEvent::InitWorker(worker_index, tetris_count);
        send_back(Msg::new(event, None, None, None));
    }

    fn init_tetris(worker_index: u8, tetris_index: u32) {
        if worker_guard(worker_index) {
            return;
        }

        let ref mut tetris = TETRIS.lock().unwrap()[tetris_index as usize];
        tetris.init();

        send_back(Msg::new(
            AppEvent::InitTetris(worker_index, tetris_index),
            Some(tetris.get_block()),
            None,
            Some(tetris.scheme.clone()),
        ));
    }

    fn tick_event(worker_index: u8, tetris_index: u32) {
        if worker_guard(worker_index) {
            return;
        }

        let ref mut tetris = TETRIS.lock().unwrap()[tetris_index as usize];
        tetris.tick();

        send_back(Msg::new(
            AppEvent::Tick(worker_index, tetris_index),
            Some(tetris.get_block()),
            Some(tetris.get_grid()),
            Some(tetris.scheme.clone()),
        ));
    }

    fn user_event(worker_index: u8, tetris_index: u32, block_events: Option<Vec<BlockEvent>>) {
        if worker_guard(worker_index) {
            return;
        }

        let ref mut tetris = TETRIS.lock().unwrap()[tetris_index as usize];
        tetris.event(block_events);

        send_back(Msg::new(
            AppEvent::User(worker_index, tetris_index, None),
            Some(tetris.get_block()),
            Some(tetris.get_grid()),
            Some(tetris.scheme.clone()),
        ));
    }

    #[no_mangle]
    pub fn on(data: *mut c_char, size: c_int) {
        let app_events = String::from_utf8(into_raw(data, size).to_vec()).unwrap();
        match serde_json::from_str::<AppEvent>(app_events.as_str()).unwrap() {
            AppEvent::InitWorker(worker_index, tetris_count) => {
                init_worker(worker_index, tetris_count)
            }
            AppEvent::InitTetris(worker_index, tetris_index) => {
                init_tetris(worker_index, tetris_index)
            }
            AppEvent::Tick(worker_index, tetris_index) => tick_event(worker_index, tetris_index),
            AppEvent::User(worker_index, tetris_index, block_event) => {
                user_event(worker_index, tetris_index, block_event)
            }
        }
    }
}

fn main() {}
