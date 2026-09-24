# RDPM (Remote Desktop Profile Manager) 实施计划

本实施计划旨在依据架构设计文档，以测试驱动和模块化演进方式，从零完成 RDPM 的完整开发与验证。

---

## 阶段规划概览

| 阶段 | 核心目标 | 产出物 |
| :--- | :--- | :--- |
| **阶段一：工程基础与核心数据模型** | 建立 Cargo 工作区，定义服务器树、凭据加密与配置存取 | `Cargo.toml`, `crates/rdpm_core`, `crates/rdpm_vault` |
| **阶段二：Windows Native RDP 宿主** | 实现 C++/ATL Shim，桥接微软 `mstscax.dll`，打通 Rust FFI | `crates/rdpm_rdp_host` (C++ Shim + Rust FFI) |
| **阶段三：GPUI 界面与多 Tab 容器** | 构建左侧服务器树、多 Tab 栏与 Native HWND 动态贴合 | `crates/rdpm_ui`, `src/main.rs` |
| **阶段四：会话管理与断线自动重连** | 完善会话生命周期、监听断线事件、指数退避重连机制 | `crates/rdpm_session` |
| **阶段五：工作区保存与一键环境恢复** | 实现工作区快照保存，下次启动时阶梯并发平滑拉起整个运维环境 | `crates/rdpm_workspace` |
| **阶段六：综合联调与打包交付** | 真实 Windows 环境全链路测试、优化渲染与异常处理 | 最终成品验证与发布构建 |

---

## 详细实施任务清单

### 阶段一：工程基础与核心数据模型
- [x] **任务 1.1**：创建根目录 `Cargo.toml` 工作区配置与依赖项版本声明。
- [x] **任务 1.2**：在 `crates/rdpm_core` 中实现领域模型与便携数据存储：
  - 便携路径计算模块（`paths.rs`）：通过 `std::env::current_exe()` 计算 `data/` 目录，杜绝读写 `%APPDATA%` 和注册表；
  - 服务器节点定义（`ServerNode`、`GroupNode`、`ServerTree`）；
  - RDP 连接选项参数（`RdpOptions`：分辨率、Smart Sizing、驱动器/剪贴板重定向）；
  - 工作区快照模型（`WorkspaceConfig`、`TabSnapshot`）；
  - 基于 JSON 的本地配置持久化存储（统一写入 `<exe_dir>/data/`）。
- [x] **任务 1.3**：在 `crates/rdpm_vault` 中实现 Windows DPAPI 凭据安全保管箱：
  - 基于 Windows `CryptProtectData` / `CryptUnprotectData` 的数据加密解密；
  - 凭据数据持久化至 `<exe_dir>/data/credentials.enc`；
  - 凭据的安全内存擦除（写入后即清零，防止内存残留）。

### 阶段二：Windows Native RDP 宿主模块（参考 Navop）
- [x] **任务 2.1**：在 `crates/rdpm_rdp_host` 中编写 C++/ATL 宿主底层 Shim：
  - `active_x_host.cpp`：调用 `AtlAxWinInit`，在指定父窗口下创建 `AtlAxWin` 窗口并实例化 `MsRdpClient12`；
  - `event_sink.cpp`：实现 `DMsRdpClientEvents` 的 `IDispatch` 事件接收器；
  - `host.h`：定义清晰稳定的 C ABI 函数（创建、连接、断开、隐藏、显式改变大小、聚焦等）。
- [x] **任务 2.2**：编写 `build.rs` 脚本，使用 `cc` crate 在 MSVC 环境下自动编译 C++ Shim 与 ATL 依赖。
- [x] **任务 2.3**：在 Rust 侧实现安全封装与生命周期管理（`RdpHostHandle`、事件通知回调通道）。

### 阶段三：GPUI 界面与多 Tab 容器集成
- [x] **任务 3.1**：初始化应用程序主控制器与入口流程（加载配置、便携目录初始化与工作区阶梯调度）。
- [x] **任务 3.2**：实现左侧服务器树视图（`ServerTreeView`）：
  - 树形折叠展开与节点渲染；
  - 顶部快速搜索/拼音过滤框；
  - 双击触发连接事件、右键上下文菜单（添加、编辑、删除服务器/分组）。
- [x] **任务 3.3**：实现多 Tab 标签页容器（`TabContainerView`）：

  - 支持多 Tab 动态增删、激活指示器与关闭按钮；
  - 实现 Airspace 协同算法：在 GPUI 布局后精准计算并同步 Native Child HWND 的物理像素尺寸与位置；
  - Tab 切换时的可见性（Show/Hide）控制与焦点转交机制。

### 阶段四：会话状态机与断线自动重连
- [x] **任务 4.1**：在 `crates/rdpm_session` 中实现会话状态机（未连接、连接中、已连接、重连中、已断开）。
- [x] **任务 4.2**：处理断线原因判定，过滤用户主动关闭与异常掉线。
- [x] **任务 4.3**：实现指数退避重连算法（1s -> 2s -> 4s -> 8s -> 15s），并在 Tab 和状态栏上呈现动态重连进度条与旋转动画。

### 阶段五：Workspace 保存与一键恢复运维环境
- [x] **任务 5.1**：在 `crates/rdpm_workspace` 中实现当前工作区状态采集器（捕获当前打开的全部 Server ID、排序及当前激活项）。
- [x] **任务 5.2**：实现工作区存盘管理（多工作区配置切换、设为默认工作区、退出自动保存）。
- [x] **任务 5.3**：实现阶梯式并发恢复调度器（Staggered Queue）：
  - 优先秒级连接恢复上次活动的 Tab；
  - 后台每隔 250ms 平滑拉起其余 Tab 会话，防止瞬时并发打满网络和系统资源。


### 阶段六：综合验证与体验调优
- [ ] **任务 6.1**：多分辨率与高 DPI 缩放场景验证（4K 150%、1080P 100% 切换拉伸）。
- [ ] **任务 6.2**：断网与网络波动恢复实测。
- [ ] **任务 6.3**：异常关闭与多进程/窗口资源清理验证，杜绝句柄泄漏。
