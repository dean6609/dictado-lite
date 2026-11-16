//! One HTTPS GET streamed to a callback. WinHTTP on Windows; curl elsewhere,
//! so the Unix builds carry no TLS stack of their own.
use crate::engine::{Error, Result};

pub const HOST: &str = "huggingface.co";

/// Fetch `path` from the model host, handing every chunk to `sink` in order.
/// A sink error (for example a cancellation) stops the transfer at once.
pub fn get(path: &str, sink: &mut dyn FnMut(&[u8]) -> Result<()>) -> Result<()> {
    #[cfg(windows)]
    {
        winhttp(path, sink)
    }
    #[cfg(not(windows))]
    {
        curl(path, sink)
    }
}

#[cfg(windows)]
fn winhttp(path: &str, sink: &mut dyn FnMut(&[u8]) -> Result<()>) -> Result<()> {
    use std::ffi::c_void;
    use windows::{
        core::{w, PCWSTR},
        Win32::Networking::WinHttp::*,
    };
    struct Internet(*mut c_void);
    impl Internet {
        fn new(ptr: *mut c_void) -> Result<Self> {
            if ptr.is_null() {
                Err(Error::message(windows::core::Error::from_win32()))
            } else {
                Ok(Self(ptr))
            }
        }
    }
    impl Drop for Internet {
        fn drop(&mut self) {
            unsafe {
                let _ = WinHttpCloseHandle(self.0);
            }
        }
    }
    let host: Vec<u16> = HOST.encode_utf16().chain(Some(0)).collect();
    let url: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let session = Internet::new(WinHttpOpen(
            w!("DictadoLite"),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        ))?;
        WinHttpSetTimeouts(session.0, 15_000, 15_000, 15_000, 15_000).map_err(Error::message)?;
        let connection = Internet::new(WinHttpConnect(session.0, PCWSTR(host.as_ptr()), 443, 0))?;
        let request = Internet::new(WinHttpOpenRequest(
            connection.0,
            w!("GET"),
            PCWSTR(url.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        ))?;
        WinHttpSendRequest(request.0, None, None, 0, 0, 0).map_err(Error::message)?;
        WinHttpReceiveResponse(request.0, std::ptr::null_mut()).map_err(Error::message)?;
        let mut status = 0u32;
        let mut bytes = 4;
        let mut index = 0;
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some((&mut status as *mut u32).cast()),
            &mut bytes,
            &mut index,
        )
        .map_err(Error::message)?;
        if status != 200 {
            return Err(Error::message(format!("Model download HTTP {status}")));
        }
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let mut read = 0;
            WinHttpReadData(
                request.0,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut read,
            )
            .map_err(Error::message)?;
            if read == 0 {
                return Ok(());
            }
            sink(&buffer[..read as usize])?;
        }
    }
}

#[cfg(not(windows))]
fn curl(path: &str, sink: &mut dyn FnMut(&[u8]) -> Result<()>) -> Result<()> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let mut child = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--connect-timeout",
            "15",
            "--speed-limit",
            "1024",
            "--speed-time",
            "15",
            "--user-agent",
            "DictadoLite",
            "--output",
            "-",
        ])
        .arg(format!("https://{HOST}{path}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::message(format!("curl is required to download models: {e}")))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::message("No curl output"))?;
    let mut buffer = [0u8; 64 * 1024];
    let streamed = (|| loop {
        let read = stdout.read(&mut buffer).map_err(Error::message)?;
        if read == 0 {
            return Ok(());
        }
        sink(&buffer[..read])?;
    })();
    if streamed.is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return streamed;
    }
    let output = child.wait_with_output().map_err(Error::message)?;
    if output.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr);
        Err(Error::message(format!(
            "Model download failed: {}",
            detail.trim()
        )))
    }
}
