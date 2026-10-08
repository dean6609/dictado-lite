//! Preallocate full-format copies before replacing clipboard data.
//! Adapted from Handy's MIT snapshot; fail closed for unsupported handle types.
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::Graphics::Gdi::{
    CopyEnhMetaFileW, DeleteEnhMetaFile, DeleteObject, HENHMETAFILE, HGDIOBJ,
};
use windows::Win32::System::DataExchange::*;
use windows::Win32::System::Memory::*;
use windows::Win32::System::Ole::*;
use windows::Win32::UI::WindowsAndMessaging::{CopyImage, IMAGE_BITMAP, LR_CREATEDIBSECTION};

pub struct Open;
impl Open {
    pub fn new(hwnd: HWND) -> Result<Self, String> {
        unsafe { OpenClipboard(Some(hwnd)) }.map_err(|e| e.to_string())?;
        Ok(Self)
    }
}
impl Drop for Open {
    fn drop(&mut self) {
        let _ = unsafe { CloseClipboard() };
    }
}

enum Kind {
    Memory,
    Bitmap,
    Enhanced,
}
struct Saved {
    format: u32,
    handle: HANDLE,
    kind: Kind,
}
impl Drop for Saved {
    fn drop(&mut self) {
        if self.handle.is_invalid() {
            return;
        }
        unsafe {
            match self.kind {
                Kind::Memory => {
                    let _ = GlobalFree(Some(HGLOBAL(self.handle.0)));
                }
                Kind::Bitmap => {
                    let _ = DeleteObject(HGDIOBJ(self.handle.0));
                }
                Kind::Enhanced => {
                    let _ = DeleteEnhMetaFile(Some(HENHMETAFILE(self.handle.0)));
                }
            }
        }
    }
}
pub struct Snapshot {
    saved: Vec<Saved>,
    restoring: bool,
}
impl Snapshot {
    /// Caller holds OpenClipboard. Nothing has been removed yet.
    pub fn capture() -> Result<Self, String> {
        let mut saved = Vec::new();
        let mut format = 0;
        let mut total = 0;
        unsafe {
            loop {
                format = EnumClipboardFormats(format);
                if format == 0 {
                    break;
                }
                let handle = GetClipboardData(format).map_err(|e| e.to_string())?;
                let entry = if format == CF_BITMAP.0 as u32 || format == CF_DSPBITMAP.0 as u32 {
                    Saved {
                        format,
                        handle: CopyImage(handle, IMAGE_BITMAP, 0, 0, LR_CREATEDIBSECTION)
                            .map_err(|e| e.to_string())?,
                        kind: Kind::Bitmap,
                    }
                } else if format == CF_ENHMETAFILE.0 as u32 || format == CF_DSPENHMETAFILE.0 as u32
                {
                    let copy = CopyEnhMetaFileW(HENHMETAFILE(handle.0), None);
                    if copy.0.is_null() {
                        return Err("Enhanced clipboard metafile copy failed".into());
                    }
                    Saved {
                        format,
                        handle: HANDLE(copy.0),
                        kind: Kind::Enhanced,
                    }
                } else {
                    if [
                        CF_METAFILEPICT.0,
                        CF_DSPMETAFILEPICT.0,
                        CF_PALETTE.0,
                        CF_OWNERDISPLAY.0,
                    ]
                    .contains(&(format as u16))
                    {
                        return Err("Clipboard format cannot be safely preserved".into());
                    }
                    let source = HGLOBAL(handle.0);
                    let size = GlobalSize(source);
                    total += size;
                    if size == 0 || total > 64 * 1024 * 1024 {
                        return Err("Clipboard data cannot be safely preserved".into());
                    }
                    let copied = GlobalAlloc(GMEM_MOVEABLE, size).map_err(|e| e.to_string())?;
                    let entry = Saved {
                        format,
                        handle: HANDLE(copied.0),
                        kind: Kind::Memory,
                    };
                    let input = GlobalLock(source);
                    let output = GlobalLock(copied);
                    if !input.is_null() && !output.is_null() {
                        std::ptr::copy_nonoverlapping(
                            input.cast::<u8>(),
                            output.cast::<u8>(),
                            size,
                        );
                    }
                    if !input.is_null() {
                        let _ = GlobalUnlock(source);
                    }
                    if !output.is_null() {
                        let _ = GlobalUnlock(copied);
                    }
                    if input.is_null() || output.is_null() {
                        return Err("Clipboard copy failed".into());
                    }
                    entry
                };
                saved.push(entry);
            }
        }
        Ok(Self {
            saved,
            restoring: false,
        })
    }
    /// Caller holds OpenClipboard and has checked sequence/ownership.
    pub fn restore(&mut self) -> Result<(), String> {
        if !self.restoring {
            unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
            self.restoring = true;
        }
        for entry in &mut self.saved {
            if entry.handle.is_invalid() {
                continue;
            }
            unsafe { SetClipboardData(entry.format, Some(entry.handle)) }
                .map_err(|e| e.to_string())?;
            entry.handle = HANDLE::default(); // Windows owns successfully restored handles.
        }
        Ok(())
    }
}

pub fn publish_bytes(format: u32, bytes: &[u8]) -> Result<(), String> {
    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len()).map_err(|e| e.to_string())?;
        let mut saved = Saved {
            format,
            handle: HANDLE(handle.0),
            kind: Kind::Memory,
        };
        let memory = GlobalLock(handle);
        if memory.is_null() {
            return Err("Clipboard allocation failed".into());
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), memory.cast::<u8>(), bytes.len());
        let _ = GlobalUnlock(handle);
        SetClipboardData(format, Some(saved.handle)).map_err(|e| e.to_string())?;
        saved.handle = HANDLE::default();
    }
    Ok(())
}
