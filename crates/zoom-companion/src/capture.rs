//! Zoom window discovery and capture.
use crate::proof::Frame;

#[derive(Clone, Debug, serde::Serialize)]
pub(crate) struct WindowCandidate {
    pub hwnd: isize, pub pid: u32, pub process: String, pub class: String, pub title: String,
    pub width: i32, pub height: i32, pub minimized: bool, pub visible: bool,
}

const MEETING_CLASSES: &[&str] = &["ConfMultiTabContentWndClass", "ZPContentViewWndClass", "VideoFrameWnd"];

pub(crate) fn pick_meeting_window(cands: &[WindowCandidate]) -> Option<&WindowCandidate> {
    cands.iter().filter(|c| c.visible && !c.minimized && MEETING_CLASSES.iter().any(|k| c.class.eq_ignore_ascii_case(k)))
        .max_by_key(|c| c.width.max(0) as i64 * c.height.max(0) as i64)
        .or_else(|| cands.iter().filter(|c| c.visible && !c.minimized && !c.title.trim().is_empty() && c.width >= 400 && c.height >= 300)
            .max_by_key(|c| c.width as i64 * c.height as i64))
        .or_else(|| cands.iter().filter(|c| c.visible && MEETING_CLASSES.iter().any(|k| c.class.eq_ignore_ascii_case(k)))
            .max_by_key(|c| c.width.max(0) as i64 * c.height.max(0) as i64))
        .or_else(|| cands.iter().filter(|c| c.visible && !c.title.trim().is_empty() && c.width >= 400 && c.height >= 300)
            .max_by_key(|c| c.width as i64 * c.height as i64))
}

#[derive(Debug)]
pub(crate) enum CaptureError { NotFound, Minimized, Unsupported, Timeout, Failed(String) }

/// Copy tightly packed BGRA rows out of a possibly padded GPU staging buffer.
fn copy_rows(src: &[u8], row_pitch: usize, width: u32, height: u32) -> Vec<u8> {
    let row_bytes = width as usize * 4;
    let mut dst = vec![0; row_bytes * height as usize];
    for y in 0..height as usize {
        let from = y * row_pitch;
        let to = y * row_bytes;
        if from + row_bytes <= src.len() { dst[to..to + row_bytes].copy_from_slice(&src[from..from + row_bytes]); }
    }
    dst
}

#[cfg(test)]
mod copy_tests {
    use super::copy_rows;
    #[test]
    fn copies_padded_bgra_rows() {
        let input = [1,2,3,4,5,6,7,8,99,99, 9,10,11,12,13,14,15,16,88,88];
        assert_eq!(copy_rows(&input, 10, 2, 2), [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
    }
}

#[cfg(windows)]
pub(crate) fn list_zoom_windows() -> Vec<WindowCandidate> {
    use std::mem::zeroed;
    use windows_sys::Win32::{Foundation::*, System::{Threading::*, ProcessStatus::*}, UI::WindowsAndMessaging::*};
    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        use windows_sys::Win32::{Foundation::*, System::{Threading::*, ProcessStatus::*}, UI::WindowsAndMessaging::*};
        let list = &mut *(lparam as *mut Vec<WindowCandidate>);
        let mut pid = 0; GetWindowThreadProcessId(hwnd, &mut pid);
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() { return 1; }
        let mut path = [0u16; 1024]; let mut len = path.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut len);
        CloseHandle(process);
        if ok == 0 { return 1; }
        let full = String::from_utf16_lossy(&path[..len as usize]);
        let name = full.rsplit(['\\','/']).next().unwrap_or("").to_owned();
        if !name.eq_ignore_ascii_case("Zoom.exe") { return 1; }
        let mut rect: RECT = zeroed(); GetWindowRect(hwnd, &mut rect);
        let mut cls = [0u16; 256]; let cn = GetClassNameW(hwnd, cls.as_mut_ptr(), cls.len() as i32);
        let mut title = [0u16; 1024]; let tn = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
        list.push(WindowCandidate { hwnd: hwnd as isize, pid, process: name, class: String::from_utf16_lossy(&cls[..cn.max(0) as usize]), title: String::from_utf16_lossy(&title[..tn.max(0) as usize]), width: rect.right-rect.left, height: rect.bottom-rect.top, minimized: IsIconic(hwnd)!=0, visible: IsWindowVisible(hwnd)!=0 }); 1
    }
    let mut out=Vec::new(); unsafe { EnumWindows(Some(callback), &mut out as *mut _ as LPARAM); } out
}

#[cfg(not(windows))]
pub(crate) fn list_zoom_windows() -> Vec<WindowCandidate> { Vec::new() }

#[cfg(windows)]
pub(crate) fn capture_window(_hwnd: isize) -> Result<Frame, CaptureError> { Err(CaptureError::Failed("Windows Graphics Capture implementation pending".into())) }
#[cfg(not(windows))]
pub(crate) fn capture_window(_hwnd: isize) -> Result<Frame, CaptureError> { Err(CaptureError::Failed("capture is only available on Windows".into())) }

pub(crate) fn capture_meeting_window() -> Result<(Frame, WindowCandidate), CaptureError> {
    let list=list_zoom_windows(); let c=pick_meeting_window(&list).cloned().ok_or(CaptureError::NotFound)?;
    if c.minimized { return Err(CaptureError::Minimized); }
    let frame=capture_window(c.hwnd)?; Ok((frame,c))
}

#[cfg(test)] mod tests {
 use super::*;
 fn c(class:&str,title:&str,w:i32,h:i32,minimized:bool)->WindowCandidate { WindowCandidate{hwnd:1,pid:2,process:"Zoom.exe".into(),class:class.into(),title:title.into(),width:w,height:h,minimized,visible:true} }
 #[test] fn prefers_zoom_meeting_class_and_locale_independent_title(){ let xs=vec![c("Popup","Settings",900,700,false),c("VideoFrameWnd","Reunión 中文",500,400,false)]; assert_eq!(pick_meeting_window(&xs).unwrap().class,"VideoFrameWnd"); }
 #[test] fn selects_largest_nonempty_fallback(){ let xs=vec![c("Other","",900,700,false),c("Other","Zoom Workplace",600,500,false)]; assert_eq!(pick_meeting_window(&xs).unwrap().width,600); }
 #[test] fn minimized_class_is_last_resort(){ let xs=vec![c("ConfMultiTabContentWndClass","",1,1,true)]; assert!(pick_meeting_window(&xs).unwrap().minimized); }
}
