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
    use windows_sys::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        use windows_sys::Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*};
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

/// Captures one frame of a window with Windows Graphics Capture (works while the window is
/// partly covered; not while minimized).
#[cfg(windows)]
pub(crate) fn capture_window(hwnd: isize) -> Result<Frame, CaptureError> {
    use windows::core::{factory, Interface};
    use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
    use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
    use windows::Graphics::DirectX::DirectXPixelFormat;
    use windows::Win32::Foundation::{HMODULE, HWND};
    use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
    use windows::Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
        D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
        D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    };
    use windows::Win32::Graphics::Dxgi::IDXGIDevice;
    use windows::Win32::System::WinRT::Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess};
    use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
    use windows::Win32::UI::WindowsAndMessaging::IsIconic;

    let fail = |what: &str, e: windows::core::Error| CaptureError::Failed(format!("{what}: {e}"));
    let hwnd = HWND(hwnd as *mut core::ffi::c_void);
    unsafe {
        if IsIconic(hwnd).as_bool() {
            return Err(CaptureError::Minimized);
        }
        // Already-initialised apartments return an error code we can ignore.
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    if !GraphicsCaptureSession::IsSupported().unwrap_or(false) {
        return Err(CaptureError::Unsupported);
    }
    let mut device: Option<ID3D11Device> = None;
    let mut context: Option<ID3D11DeviceContext> = None;
    unsafe {
        D3D11CreateDevice(None, D3D_DRIVER_TYPE_HARDWARE, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut context))
            .map_err(|e| fail("D3D11CreateDevice", e))?;
    }
    let device = device.ok_or_else(|| CaptureError::Failed("no D3D11 device".into()))?;
    let context = context.ok_or_else(|| CaptureError::Failed("no D3D11 context".into()))?;
    let dxgi: IDXGIDevice = device.cast().map_err(|e| fail("IDXGIDevice", e))?;
    let d3d: IDirect3DDevice = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) }
        .and_then(|inspectable| inspectable.cast())
        .map_err(|e| fail("CreateDirect3D11DeviceFromDXGIDevice", e))?;
    let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().map_err(|e| fail("capture interop", e))?;
    let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd) }.map_err(|e| fail("CreateForWindow", e))?;
    let size = item.Size().map_err(|e| fail("item size", e))?;
    let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(&d3d, DirectXPixelFormat::B8G8R8A8UIntNormalized, 1, size)
        .map_err(|e| fail("frame pool", e))?;
    let session = pool.CreateCaptureSession(&item).map_err(|e| fail("capture session", e))?;
    let _ = session.SetIsCursorCaptureEnabled(false);
    let _ = session.SetIsBorderRequired(false); // Windows 11+; older builds show a yellow border.
    session.StartCapture().map_err(|e| fail("StartCapture", e))?;
    let started = std::time::Instant::now();
    let frame = loop {
        if let Ok(frame) = pool.TryGetNextFrame() {
            break frame;
        }
        if started.elapsed() > std::time::Duration::from_secs(2) {
            let _ = session.Close();
            let _ = pool.Close();
            return Err(CaptureError::Timeout);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let result = (|| -> Result<Frame, CaptureError> {
        let surface = frame.Surface().map_err(|e| fail("frame surface", e))?;
        let access: IDirect3DDxgiInterfaceAccess = surface.cast().map_err(|e| fail("surface access", e))?;
        let texture: ID3D11Texture2D = unsafe { access.GetInterface() }.map_err(|e| fail("surface texture", e))?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { texture.GetDesc(&mut desc) };
        let staging_desc = D3D11_TEXTURE2D_DESC {
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
            ..desc
        };
        let mut staging: Option<ID3D11Texture2D> = None;
        unsafe { device.CreateTexture2D(&staging_desc, None, Some(&mut staging)) }.map_err(|e| fail("staging texture", e))?;
        let staging = staging.ok_or_else(|| CaptureError::Failed("no staging texture".into()))?;
        unsafe { context.CopyResource(&staging, &texture) };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe { context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }.map_err(|e| fail("map", e))?;
        let pitch = mapped.RowPitch as usize;
        let bytes = unsafe { std::slice::from_raw_parts(mapped.pData as *const u8, pitch * desc.Height as usize) };
        let bgra = copy_rows(bytes, pitch, desc.Width, desc.Height);
        unsafe { context.Unmap(&staging, 0) };
        Ok(Frame { width: desc.Width, height: desc.Height, bgra })
    })();
    let _ = frame.Close();
    let _ = session.Close();
    let _ = pool.Close();
    result
}
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
    /// Real Windows Graphics Capture smoke test on the taskbar (needs a desktop session):
    /// `cargo test -p zoom-companion wgc_captures_taskbar -- --ignored`
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn wgc_captures_taskbar() {
        use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
        let class: Vec<u16> = "Shell_TrayWnd ".encode_utf16().collect();
        let hwnd = unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) } as isize;
        assert_ne!(hwnd, 0, "taskbar not found");
        let frame = super::capture_window(hwnd).expect("capture");
        assert!(frame.width > 100 && frame.height > 10, "{}x{}", frame.width, frame.height);
        assert_eq!(frame.bgra.len(), (frame.width * frame.height * 4) as usize);
        let distinct: std::collections::HashSet<_> = frame.bgra.chunks(4).take(200_000).map(|p| (p[0], p[1], p[2])).collect();
        assert!(distinct.len() > 3, "frame looks blank");
        if let Ok(dir) = std::env::var("ZC_WGC_DUMP") {
            std::fs::write(std::path::Path::new(&dir).join("taskbar.png"), crate::proof::encode_png(&frame)).unwrap();
        }
    }

}
