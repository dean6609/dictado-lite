//! Minimal safe owner over the existing MIT transcribe.cpp C ABI.
use super::{Error, Result, MODEL_BYTES};
use std::ffi::{c_char, c_void, CStr, CString};
use std::path::Path;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use transcribe_cpp_sys as sys;

#[derive(Clone, Copy, Debug)]
pub enum Backend {
    PreferVulkan,
    Cpu,
}

#[derive(Clone, Default, Debug)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

extern "C" fn abort(userdata: *mut c_void) -> bool {
    // SAFETY: run() retains the Arc until the callback is cleared.
    unsafe { (*(userdata.cast::<AtomicBool>())).load(Ordering::SeqCst) }
}

pub struct Engine {
    model: NonNull<sys::transcribe_model>,
    session: NonNull<sys::transcribe_session>,
    pub backend: String,
}
// SAFETY: unique ownership; every compute call requires &mut self. The worker
// moves the engine but never exposes it concurrently to another thread.
unsafe impl Send for Engine {}

impl Engine {
    pub fn load(path: &Path, backend: Backend) -> Result<Self> {
        if std::fs::metadata(path).map_err(Error::message)?.len() != MODEL_BYTES {
            return Err(Error::message("Unexpected Parakeet model size"));
        }
        static INIT: OnceLock<std::result::Result<(), String>> = OnceLock::new();
        let init = INIT.get_or_init(|| {
            // SAFETY: process-wide backend registration once, before model load.
            check(unsafe { sys::transcribe_init_backends_default() }).map_err(|e| e.to_string())
        });
        if let Err(error) = init {
            return Err(Error::message(error));
        }
        match backend {
            Backend::PreferVulkan => Self::load_on(
                path,
                sys::transcribe_backend_request::TRANSCRIBE_BACKEND_VULKAN,
            )
            .or_else(|_| {
                Self::load_on(
                    path,
                    sys::transcribe_backend_request::TRANSCRIBE_BACKEND_CPU,
                )
            }),
            Backend::Cpu => Self::load_on(
                path,
                sys::transcribe_backend_request::TRANSCRIBE_BACKEND_CPU,
            ),
        }
    }

    fn load_on(path: &Path, backend: sys::transcribe_backend_request) -> Result<Self> {
        let path = path
            .to_str()
            .ok_or_else(|| Error::message("Model path is not UTF-8"))?;
        let path = CString::new(path).map_err(Error::message)?;
        // SAFETY: structs are initialized by their matching ABI initializer;
        // C strings remain alive for each synchronous call; all outputs checked.
        unsafe {
            let mut params = std::mem::zeroed();
            sys::transcribe_model_load_params_init(&mut params);
            params.backend = backend;
            let mut model = std::ptr::null_mut();
            check(sys::transcribe_model_load_file(
                path.as_ptr(),
                &params,
                &mut model,
            ))?;
            let model = NonNull::new(model).ok_or_else(|| Error::message("Null model"))?;
            if string(sys::transcribe_model_arch_string(model.as_ptr())) != "parakeet" {
                sys::transcribe_model_free(model.as_ptr());
                return Err(Error::message("Only Parakeet is supported"));
            }
            let mut params = std::mem::zeroed();
            sys::transcribe_session_params_init(&mut params);
            let mut session = std::ptr::null_mut();
            if let Err(error) = check(sys::transcribe_session_init(
                model.as_ptr(),
                &params,
                &mut session,
            )) {
                sys::transcribe_model_free(model.as_ptr());
                return Err(error);
            }
            let Some(session) = NonNull::new(session) else {
                sys::transcribe_model_free(model.as_ptr());
                return Err(Error::message("Null session"));
            };
            Ok(Self {
                model,
                session,
                backend: string(sys::transcribe_model_backend(model.as_ptr())),
            })
        }
    }

    pub fn run(&mut self, samples: &[f32], cancel: &Cancellation) -> Result<String> {
        crate::audio::validate(samples)?;
        if cancel.cancelled() {
            return Err(Error::Cancelled);
        }
        let count = i32::try_from(samples.len()).map_err(Error::message)?;
        let flag = Arc::clone(&cancel.0);
        // SAFETY: session is uniquely borrowed, PCM lives through the call,
        // retained flag outlives callback, text copied before session reuse.
        unsafe {
            sys::transcribe_set_abort_callback(
                self.session.as_ptr(),
                Some(abort),
                Arc::as_ptr(&flag) as *mut c_void,
            );
            let mut params = std::mem::zeroed();
            sys::transcribe_run_params_init(&mut params);
            let status =
                sys::transcribe_run(self.session.as_ptr(), samples.as_ptr(), count, &params);
            sys::transcribe_set_abort_callback(self.session.as_ptr(), None, std::ptr::null_mut());
            if cancel.cancelled() || sys::transcribe_was_aborted(self.session.as_ptr()) {
                return Err(Error::Cancelled);
            }
            check(status)?;
            Ok(string(sys::transcribe_full_text(self.session.as_ptr())))
        }
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        // SAFETY: no in-flight run can coexist with ownership being dropped.
        unsafe {
            sys::transcribe_session_free(self.session.as_ptr());
            sys::transcribe_model_free(self.model.as_ptr());
        }
    }
}
fn check(status: sys::transcribe_status) -> Result<()> {
    if status == sys::transcribe_status::TRANSCRIBE_OK {
        Ok(())
    } else {
        // SAFETY: library returns a process-lifetime diagnostic C string.
        Err(Error::message(unsafe {
            string(sys::transcribe_status_string(status.0 as i32))
        }))
    }
}
unsafe fn string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}
