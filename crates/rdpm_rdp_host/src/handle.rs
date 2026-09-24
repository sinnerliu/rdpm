use std::ffi::c_void;
use std::os::raw::c_int;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use windows::Win32::Foundation::HWND;

use crate::ffi::*;

/// RDP 强类型事件
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RdpHostEvent {
    Connecting,
    Connected,
    LoginComplete,
    Disconnected { reason: i32, description: String },
    AutoReconnecting,
    AutoReconnected,
    FatalError { code: i32, description: String },
}

/// 宿主回调代理结构体
struct CallbackProxy {
    sender: UnboundedSender<RdpHostEvent>,
}

unsafe extern "C" fn event_callback(
    user_data: *mut c_void,
    event_type: c_int,
    reason_or_error: i32,
    description: *const u16,
) {
    if user_data.is_null() {
        return;
    }

    let proxy = &*(user_data as *const CallbackProxy);

    let desc_str = if description.is_null() {
        String::new()
    } else {
        let mut len = 0;
        while *description.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(description, len);
        String::from_utf16_lossy(slice)
    };

    let event = match event_type {
        1 => RdpHostEvent::Connecting,
        2 => RdpHostEvent::Connected,
        3 => RdpHostEvent::LoginComplete,
        4 => RdpHostEvent::Disconnected {
            reason: reason_or_error,
            description: desc_str,
        },
        5 => RdpHostEvent::AutoReconnecting,
        6 => RdpHostEvent::AutoReconnected,
        7 => RdpHostEvent::FatalError {
            code: reason_or_error,
            description: desc_str,
        },
        _ => return,
    };

    let _ = proxy.sender.send(event);
}

/// RDP 连接配置参数（Rust 原生类型）
#[derive(Debug, Clone, Default)]
pub struct HostConnectOptions {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub domain: Option<String>,
    pub password: Option<String>,
    pub desktop_width: u32,
    pub desktop_height: u32,
    pub smart_sizing: bool,
    pub audio_mode: u32,
    pub redirect_clipboard: bool,
    pub redirect_drives: bool,
    pub redirect_printers: bool,
    pub admin_session: bool,
}

/// 安全的 RDP 宿主实例
pub struct RdpHost {
    raw: *mut RdpmNativeHost,
    _proxy: Box<CallbackProxy>,
}

unsafe impl Send for RdpHost {}

impl RdpHost {
    /// 在指定的 Win32 父窗口下创建嵌入式 RDP 宿主
    pub fn create(parent_hwnd: HWND) -> Result<(Self, UnboundedReceiver<RdpHostEvent>)> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let proxy = Box::new(CallbackProxy { sender });

        let raw = unsafe {
            rdpm_host_create(
                parent_hwnd,
                event_callback,
                proxy.as_ref() as *const _ as *mut c_void,
            )
        };

        if raw.is_null() {
            return Err(anyhow!("创建 RDP ActiveX 宿主失败"));
        }

        Ok((Self { raw, _proxy: proxy }, receiver))
    }

    /// 发起连接
    pub fn connect(&self, options: &HostConnectOptions) -> Result<()> {
        let to_wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };

        let host_w = to_wide(&options.host);
        let username_w = to_wide(&options.username);
        let domain_w = options.domain.as_ref().map(|d| to_wide(d));
        let password_w = options.password.as_ref().map(|p| to_wide(p));

        let params = RdpmConnectParams {
            host: host_w.as_ptr(),
            port: options.port as u32,
            username: username_w.as_ptr(),
            domain: domain_w.as_ref().map(|d| d.as_ptr()).unwrap_or(std::ptr::null()),
            password: password_w.as_ref().map(|p| p.as_ptr()).unwrap_or(std::ptr::null()),
            desktop_width: options.desktop_width,
            desktop_height: options.desktop_height,
            smart_sizing: if options.smart_sizing { 1 } else { 0 },
            audio_mode: options.audio_mode,
            redirect_clipboard: if options.redirect_clipboard { 1 } else { 0 },
            redirect_drives: if options.redirect_drives { 1 } else { 0 },
            redirect_printers: if options.redirect_printers { 1 } else { 0 },
            admin_session: if options.admin_session { 1 } else { 0 },
        };


        let res = unsafe { rdpm_host_connect(self.raw, &params) };
        if res != 0 {
            return Err(anyhow!("发起 RDP 连接失败，错误码: {}", res));
        }

        Ok(())
    }

    /// 断开连接
    pub fn disconnect(&self) -> Result<()> {
        let res = unsafe { rdpm_host_disconnect(self.raw) };
        if res != 0 {
            return Err(anyhow!("断开 RDP 连接失败，错误码: {}", res));
        }
        Ok(())
    }

    /// 更新子窗口物理像素尺寸与位置
    pub fn set_bounds(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            rdpm_host_set_bounds(self.raw, x, y, width, height);
        }
    }

    /// 显示或隐藏子窗口
    pub fn set_visible(&self, visible: bool) {
        unsafe {
            rdpm_host_set_visible(self.raw, if visible { 1 } else { 0 });
        }
    }

    /// 将焦点转移给 RDP 控件
    pub fn set_focus(&self) {
        unsafe {
            rdpm_host_set_focus(self.raw);
        }
    }

    /// 动态更新分辨率
    pub fn update_display(&self, width: u32, height: u32) {
        unsafe {
            rdpm_host_update_display(self.raw, width, height);
        }
    }
}

impl Drop for RdpHost {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                rdpm_host_destroy(self.raw);
            }
            self.raw = std::ptr::null_mut();
        }
    }
}
