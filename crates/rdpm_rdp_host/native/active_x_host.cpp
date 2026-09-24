#include "host.h"

#include <windows.h>
#include <atlbase.h>
#include <atlhost.h>
#include <ocidl.h>

#pragma warning(push)
#pragma warning(disable : 4471)
#include "mstscax.tlh"
#pragma warning(pop)


// ATL 模块支持
class RdpmAtlModule final : public CAtlModuleT<RdpmAtlModule> {};
static RdpmAtlModule g_rdpm_atl_module;

// RDP 事件接收器 (Event Sink)
class RdpmEventSink : public IDispatch {
public:
    RdpmEventSink(RdpmEventCallback cb, void* user_data)
        : m_ref_count(1), m_cb(cb), m_user_data(user_data), m_cookie(0) {}

    // IUnknown 接口
    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override {
        if (!ppv) return E_POINTER;
        if (riid == IID_IUnknown || riid == IID_IDispatch || riid == DIID_DMsRdpClientEvents) {
            *ppv = static_cast<IDispatch*>(this);
            AddRef();
            return S_OK;
        }
        *ppv = nullptr;
        return E_NOINTERFACE;
    }

    STDMETHODIMP_(ULONG) AddRef() override {
        return InterlockedIncrement(&m_ref_count);
    }

    STDMETHODIMP_(ULONG) Release() override {
        ULONG count = InterlockedDecrement(&m_ref_count);
        if (count == 0) {
            delete this;
        }
        return count;
    }

    // IDispatch 接口
    STDMETHODIMP GetTypeInfoCount(UINT* pctinfo) override { return E_NOTIMPL; }
    STDMETHODIMP GetTypeInfo(UINT iTInfo, LCID lcid, ITypeInfo** ppTInfo) override { return E_NOTIMPL; }
    STDMETHODIMP GetIDsOfNames(REFIID riid, LPOLESTR* rgszNames, UINT cNames, LCID lcid, DISPID* rgDispId) override { return E_NOTIMPL; }

    STDMETHODIMP Invoke(
        DISPID dispIdMember,
        REFIID riid,
        LCID lcid,
        WORD wFlags,
        DISPPARAMS* pDispParams,
        VARIANT* pVarResult,
        EXCEPINFO* pExcepInfo,
        UINT* puArgErr
    ) override {
        if (!m_cb) return S_OK;

        switch (dispIdMember) {
            case 1: // OnConnecting
                m_cb(m_user_data, RDPM_EVENT_CONNECTING, 0, L"正在连接远程桌面...");
                break;
            case 2: // OnConnected
                m_cb(m_user_data, RDPM_EVENT_CONNECTED, 0, L"网络连接已建立");
                break;
            case 3: // OnLoginComplete
                m_cb(m_user_data, RDPM_EVENT_LOGIN_COMPLETE, 0, L"远程登录就绪");
                break;
            case 4: { // OnDisconnected(long discReason)
                long reason = 0;
                if (pDispParams && pDispParams->cArgs > 0 && pDispParams->rgvarg[0].vt == VT_I4) {
                    reason = pDispParams->rgvarg[0].lVal;
                }
                m_cb(m_user_data, RDPM_EVENT_DISCONNECTED, reason, L"会话已断开");
                break;
            }
            case 12: // OnAutoReconnecting
                m_cb(m_user_data, RDPM_EVENT_AUTO_RECONNECTING, 0, L"检测到网络闪断，正在自动重连...");
                break;
            case 13: // OnAutoReconnected
                m_cb(m_user_data, RDPM_EVENT_AUTO_RECONNECTED, 0, L"自动重连成功");
                break;
            case 7: { // OnFatalError(long errorCode)
                long code = 0;
                if (pDispParams && pDispParams->cArgs > 0 && pDispParams->rgvarg[0].vt == VT_I4) {
                    code = pDispParams->rgvarg[0].lVal;
                }
                m_cb(m_user_data, RDPM_EVENT_FATAL_ERROR, code, L"发生不可恢复严重错误");
                break;
            }
            default:
                break;
        }
        return S_OK;
    }

    DWORD m_cookie;

private:
    volatile ULONG m_ref_count;
    RdpmEventCallback m_cb;
    void* m_user_data;
};

// 宿主实例内部结构体
struct RdpmNativeHost {
    HWND m_parent_hwnd;
    HWND m_container_hwnd;
    IMsRdpClient10* m_client;
    IMsRdpClientNonScriptable5* m_non_scriptable;
    RdpmEventSink* m_sink;
    DWORD m_advise_cookie;

    RdpmNativeHost()
        : m_parent_hwnd(nullptr),
          m_container_hwnd(nullptr),
          m_client(nullptr),
          m_non_scriptable(nullptr),
          m_sink(nullptr),
          m_advise_cookie(0) {}
};

// 初始化 ATL
static void ensure_atl_initialized() {
    static bool s_inited = false;
    if (!s_inited) {
        OleInitialize(nullptr);
        AtlAxWinInit();
        s_inited = true;
    }
}

extern "C" {

RdpmNativeHost* rdpm_host_create(HWND parent_hwnd, RdpmEventCallback cb, void* user_data) {
    ensure_atl_initialized();

    RdpmNativeHost* host = new (std::nothrow) RdpmNativeHost();
    if (!host) return nullptr;

    host->m_parent_hwnd = parent_hwnd;

    // 创建 ATL AxWin 子窗口（初始为隐藏状态）
    host->m_container_hwnd = CreateWindowExW(
        0,
        L"AtlAxWin",
        L"MsRdpClient12", // 优先使用系统最先进的 MsRdpClient12
        WS_CHILD | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
        0, 0, 0, 0,
        parent_hwnd,
        nullptr,
        GetModuleHandle(nullptr),
        nullptr
    );

    if (!host->m_container_hwnd) {
        // 若系统未注册 MsRdpClient12，尝试回退到通用 ProgID
        host->m_container_hwnd = CreateWindowExW(
            0,
            L"AtlAxWin",
            L"MsTscAx.MsTscAx",
            WS_CHILD | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
            0, 0, 0, 0,
            parent_hwnd,
            nullptr,
            GetModuleHandle(nullptr),
            nullptr
        );
    }

    if (!host->m_container_hwnd) {
        delete host;
        return nullptr;
    }

    // 获取控件接口
    CComPtr<IUnknown> unk;
    if (FAILED(AtlAxGetControl(host->m_container_hwnd, &unk)) || !unk) {
        DestroyWindow(host->m_container_hwnd);
        delete host;
        return nullptr;
    }

    // 查询高版本 IMsRdpClient10
    if (FAILED(unk->QueryInterface(__uuidof(IMsRdpClient10), (void**)&host->m_client))) {
        // 尝试降级查询基础 IMsRdpClient
        if (FAILED(unk->QueryInterface(__uuidof(IMsRdpClient), (void**)&host->m_client))) {
            DestroyWindow(host->m_container_hwnd);
            delete host;
            return nullptr;
        }
    }

    // 查询非脚本接口（用于安全写入密码）
    unk->QueryInterface(__uuidof(IMsRdpClientNonScriptable5), (void**)&host->m_non_scriptable);

    // 挂接事件
    if (cb) {
        host->m_sink = new RdpmEventSink(cb, user_data);
        AtlAdvise(host->m_client, host->m_sink, DIID_DMsRdpClientEvents, &host->m_advise_cookie);
    }

    return host;
}

int32_t rdpm_host_connect(RdpmNativeHost* host, const RdpmConnectParams* params) {
    if (!host || !host->m_client || !params) return -1;

    // 1. 设置 Server
    if (params->host) {
        BSTR bstr_server = SysAllocString(params->host);
        host->m_client->put_Server(bstr_server);
        SysFreeString(bstr_server);
    }

    // 2. 设置 UserName
    if (params->username) {
        BSTR bstr_user = SysAllocString(params->username);
        host->m_client->put_UserName(bstr_user);
        SysFreeString(bstr_user);
    }

    // 3. 设置 Domain
    if (params->domain && wcslen(params->domain) > 0) {
        BSTR bstr_domain = SysAllocString(params->domain);
        host->m_client->put_Domain(bstr_domain);
        SysFreeString(bstr_domain);
    }

    // 4. 设置只写密码（写入后即刻清空）
    if (params->password && host->m_non_scriptable) {
        BSTR bstr_pass = SysAllocString(params->password);
        host->m_non_scriptable->put_ClearTextPassword(bstr_pass);
        SecureZeroMemory(bstr_pass, SysStringByteLen(bstr_pass));
        SysFreeString(bstr_pass);
    }

    // 5. 设置分辨率与显示模式
    if (params->desktop_width > 0 && params->desktop_height > 0) {
        host->m_client->put_DesktopWidth(static_cast<long>(params->desktop_width));
        host->m_client->put_DesktopHeight(static_cast<long>(params->desktop_height));
    }

    // 6. 配置高级设置 (AdvancedSettings)
    CComPtr<IMsRdpClientAdvancedSettings> adv;
    if (SUCCEEDED(host->m_client->get_AdvancedSettings(&adv)) && adv) {
        adv->put_RdpPort(static_cast<long>(params->port ? params->port : 3389));
        adv->put_SmartSizing(params->smart_sizing ? VARIANT_TRUE : VARIANT_FALSE);
        adv->put_RedirectDrives(params->redirect_drives ? VARIANT_TRUE : VARIANT_FALSE);
        adv->put_RedirectPrinters(params->redirect_printers ? VARIANT_TRUE : VARIANT_FALSE);
        adv->put_RedirectClipboard(params->redirect_clipboard ? VARIANT_TRUE : VARIANT_FALSE);
    }

    // 7. 发起连接
    HRESULT hr = host->m_client->Connect();
    return SUCCEEDED(hr) ? 0 : -2;
}

int32_t rdpm_host_disconnect(RdpmNativeHost* host) {
    if (!host || !host->m_client) return -1;
    HRESULT hr = host->m_client->Disconnect();
    return SUCCEEDED(hr) ? 0 : -2;
}

void rdpm_host_destroy(RdpmNativeHost* host) {
    if (!host) return;

    if (host->m_sink && host->m_client && host->m_advise_cookie != 0) {
        AtlUnadvise(host->m_client, DIID_DMsRdpClientEvents, host->m_advise_cookie);
        host->m_sink->Release();
        host->m_sink = nullptr;
    }

    if (host->m_non_scriptable) {
        host->m_non_scriptable->Release();
        host->m_non_scriptable = nullptr;
    }

    if (host->m_client) {
        host->m_client->Disconnect();
        host->m_client->Release();
        host->m_client = nullptr;
    }

    if (host->m_container_hwnd && IsWindow(host->m_container_hwnd)) {
        DestroyWindow(host->m_container_hwnd);
        host->m_container_hwnd = nullptr;
    }

    delete host;
}

void rdpm_host_set_bounds(RdpmNativeHost* host, int32_t x, int32_t y, int32_t width, int32_t height) {
    if (!host || !host->m_container_hwnd || !IsWindow(host->m_container_hwnd)) return;
    SetWindowPos(
        host->m_container_hwnd,
        HWND_TOP,
        x, y, width, height,
        SWP_NOACTIVATE | SWP_NOZORDER
    );
}

void rdpm_host_set_visible(RdpmNativeHost* host, int32_t visible) {
    if (!host || !host->m_container_hwnd || !IsWindow(host->m_container_hwnd)) return;
    ShowWindow(host->m_container_hwnd, visible ? SW_SHOW : SW_HIDE);
}

void rdpm_host_set_focus(RdpmNativeHost* host) {
    if (!host || !host->m_container_hwnd || !IsWindow(host->m_container_hwnd)) return;
    SetFocus(host->m_container_hwnd);
}

void rdpm_host_update_display(RdpmNativeHost* host, uint32_t width, uint32_t height) {
    if (!host || !host->m_client || width == 0 || height == 0) return;
    CComPtr<IMsRdpClient8> client8;
    if (SUCCEEDED(host->m_client->QueryInterface(__uuidof(IMsRdpClient8), (void**)&client8)) && client8) {
        // 调用 RDP 8.1+ 动态重绘无感切换分辨率
        client8->UpdateSessionDisplaySettings(
            width,
            height,
            width,
            height,
            0,
            100,
            100
        );
    }
}

} // extern "C"
