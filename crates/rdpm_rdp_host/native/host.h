#pragma once

#include <stdint.h>
#include <windows.h>

#ifdef __cplusplus
extern "C" {
#endif

// 宿主实例不透明句柄
typedef struct RdpmNativeHost RdpmNativeHost;

// RDP 事件枚举
typedef enum {
    RDPM_EVENT_CONNECTING = 1,          // 正在连接
    RDPM_EVENT_CONNECTED = 2,           // 网络已建立
    RDPM_EVENT_LOGIN_COMPLETE = 3,      // 登录就绪，开始渲染画面
    RDPM_EVENT_DISCONNECTED = 4,        // 已断开
    RDPM_EVENT_AUTO_RECONNECTING = 5,   // 正在自动重连
    RDPM_EVENT_AUTO_RECONNECTED = 6,    // 自动重连成功
    RDPM_EVENT_FATAL_ERROR = 7          // 发生严重不可恢复错误
} RdpmEventType;

// 事件回调函数原型
typedef void (*RdpmEventCallback)(
    void* user_data,
    int32_t event_type,
    int32_t reason_or_error,
    const wchar_t* description
);

// RDP 连接参数
typedef struct {
    const wchar_t* host;
    uint32_t port;
    const wchar_t* username;
    const wchar_t* domain;
    const wchar_t* password;
    uint32_t desktop_width;
    uint32_t desktop_height;
    uint32_t smart_sizing;
    uint32_t audio_mode;         // 0: 本地, 1: 远端, 2: 禁用
    uint32_t redirect_clipboard; // 0: 关, 1: 开
    uint32_t redirect_drives;    // 0: 关, 1: 开
    uint32_t admin_session;      // 0: 关, 1: 开
} RdpmConnectParams;

// 创建 RDP 宿主实例并在父窗口下创建嵌入的子窗口
RdpmNativeHost* rdpm_host_create(HWND parent_hwnd, RdpmEventCallback cb, void* user_data);

// 发起 RDP 连接
int32_t rdpm_host_connect(RdpmNativeHost* host, const RdpmConnectParams* params);

// 断开 RDP 连接
int32_t rdpm_host_disconnect(RdpmNativeHost* host);

// 彻底销毁宿主并释放 COM 资源
void rdpm_host_destroy(RdpmNativeHost* host);

// 更新宿主子窗口的物理像素坐标与尺寸
void rdpm_host_set_bounds(RdpmNativeHost* host, int32_t x, int32_t y, int32_t width, int32_t height);

// 设置子窗口可见性（1: 显示 SW_SHOW, 0: 隐藏 SW_HIDE）
void rdpm_host_set_visible(RdpmNativeHost* host, int32_t visible);

// 将键盘和输入焦点设置到 RDP 控件内部
void rdpm_host_set_focus(RdpmNativeHost* host);

// 动态更新远程分辨率
void rdpm_host_update_display(RdpmNativeHost* host, uint32_t width, uint32_t height);

#ifdef __cplusplus
}
#endif
