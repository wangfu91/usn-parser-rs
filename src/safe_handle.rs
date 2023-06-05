use windows::Win32::Foundation;

#[derive(Debug)]
pub struct SafeHandle(pub Foundation::HANDLE);

impl Drop for SafeHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            if let Err(err) = unsafe { Foundation::CloseHandle(self.0) }.ok() {
                eprintln!("[Error] Failed to close handle: {}", err);
            }
            self.0 = Foundation::HANDLE::default();
        }
    }
}
