use std::ffi::c_void;
use std::os::raw::c_int;
use windows::Win32::Foundation::HWND;

#[repr(C)]
pub struct RdpmNativeHost {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdpmEventType {
    Connecting = 1,
    Connected = 2,
    LoginComplete = 3,
    Disconnected = 4,
    AutoReconnecting = 5,
    AutoReconnected = 6,
    FatalError = 7,
}

pub type RdpmEventCallback = unsafe extern "C" fn(
    user_data: *mut c_void,
    event_type: c_int,
    reason_or_error: i32,
    description: *const u16,
);

#[repr(C)]
pub struct RdpmConnectParams {
    pub host: *const u16,
    pub port: u32,
    pub username: *const u16,
    pub domain: *const u16,
    pub password: *const u16,
    pub desktop_width: u32,
    pub desktop_height: u32,
    pub smart_sizing: u32,
    pub audio_mode: u32,
    pub redirect_clipboard: u32,
    pub redirect_drives: u32,
    pub redirect_printers: u32,
    pub admin_session: u32,
}


#[cfg(all(windows, target_env = "msvc"))]
extern "C" {
    pub fn rdpm_host_create(
        parent_hwnd: HWND,
        cb: RdpmEventCallback,
        user_data: *mut c_void,
    ) -> *mut RdpmNativeHost;

    pub fn rdpm_host_connect(
        host: *mut RdpmNativeHost,
        params: *const RdpmConnectParams,
    ) -> i32;

    pub fn rdpm_host_disconnect(host: *mut RdpmNativeHost) -> i32;

    pub fn rdpm_host_destroy(host: *mut RdpmNativeHost);

    pub fn rdpm_host_set_bounds(
        host: *mut RdpmNativeHost,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    );

    pub fn rdpm_host_set_visible(host: *mut RdpmNativeHost, visible: i32);

    pub fn rdpm_host_set_focus(host: *mut RdpmNativeHost);

    pub fn rdpm_host_update_display(host: *mut RdpmNativeHost, width: u32, height: u32);
}

// 模拟回退 stub 实现（用于非 Windows/非 MSVC 环境静态检查）
#[cfg(not(all(windows, target_env = "msvc")))]
pub unsafe fn rdpm_host_create(
    _parent_hwnd: HWND,
    _cb: RdpmEventCallback,
    _user_data: *mut c_void,
) -> *mut RdpmNativeHost {
    std::ptr::null_mut()
}

#[cfg(not(all(windows, target_env = "msvc")))]
pub unsafe fn rdpm_host_connect(
    _host: *mut RdpmNativeHost,
    _params: *const RdpmConnectParams,
) -> i32 {
    0
}

#[cfg(not(all(windows, target_env = "msvc")))]
pub unsafe fn rdpm_host_disconnect(_host: *mut RdpmNativeHost) -> i32 {
    0
}

#[cfg(not(all(windows, target_env = "msvc")))]
pub unsafe fn rdpm_host_destroy(_host: *mut RdpmNativeHost) {}

#[cfg(not(all(windows, target_env = "msvc")))]
pub unsafe fn rdpm_host_set_bounds(
    _host: *mut RdpmNativeHost,
    _x: i32,
    _y: i32,
    _width: i32,
    _height: i32,
) {}

#[cfg(not(all(windows, target_env = "msvc")))]
pub unsafe fn rdpm_host_set_visible(_host: *mut RdpmNativeHost, _visible: i32) {}

#[cfg(not(all(windows, target_env = "msvc")))]
pub unsafe fn rdpm_host_set_focus(_host: *mut RdpmNativeHost) {}

#[cfg(not(all(windows, target_env = "msvc")))]
pub unsafe fn rdpm_host_update_display(_host: *mut RdpmNativeHost, _width: u32, _height: u32) {}
