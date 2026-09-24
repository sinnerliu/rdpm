# RDPM (Remote Desktop Profile Manager) 阶段说明与交付标准

本文档明确 RDPM 项目从工程构建到最终交付的阶段分解、工作目标、交付物清单及准入准出标准（Definition of Done, DoD）。

---

## 阶段规划全景表

| 阶段编号 | 阶段名称 | 核心重点 | 预估产出 |
| :---: | :--- | :--- | :--- |
| **阶段一** | **工程骨架与基础数据模型** | Workspace 搭建、服务器树模型、DPAPI 凭据保险箱 | `rdpm_core`, `rdpm_vault` |
| **阶段二** | **Windows 原生 RDP 宿主 (PoC)** | C++/ATL Shim、mstscax.dll 桥接、C ABI 与 Rust FFI | `rdpm_rdp_host` |
| **阶段三** | **GPUI 界面与多 Tab 融合** | 侧边栏服务器树、多 Tab 容器、Airspace 几何贴合 | `rdpm_ui`, 主程序入口 |
| **阶段四** | **会话状态机与断线自动重连** | 断线捕获、指数退避重试调度、会话状态流转 | `rdpm_session` |
| **阶段五** | **Workspace 快照与一键平滑恢复** | 工作区模型、状态快照存盘、阶梯并发恢复调度 | `rdpm_workspace` |
| **阶段六** | **多屏 HiDPI 适配、综合联调与交付** | 动态分辨率同步、Airspace 弹窗避让、端到端验收 | 发布版本二进制包 |

---

## 阶段详细说明与交付标准

### 阶段一：工程骨架与基础数据模型 (Phase 1)
- **目标**：完成整个项目的 Cargo Workspace 初始化，建立核心领域对象与基于 Windows DPAPI 的安全加密体系，并确立纯绿色版目录结构。
- **任务细节**：
  1. 根目录 `Cargo.toml` 配置依赖项（gpui、windows、serde、tokio、zeroize 等）；
  2. `crates/rdpm_core`：
     - 实现便携路径解析模块（`paths.rs`）：通过 `std::env::current_exe()` 解析获取当前 exe 同级目录下的 `data/` 目录，杜绝读写 `%APPDATA%` 和注册表；
     - 实现 `ServerTree`、`ServerGroup`、`ServerEntry`、`RdpOptions` 等结构定义与 JSON 格式的序列化/反序列化；
  3. `crates/rdpm_vault`：调用 Windows API `CryptProtectData` 与 `CryptUnprotectData` 实现机器绑定的密码加密，数据保存于 `<exe_dir>/data/credentials.enc`，配合 `zeroize` 保障内存安全。
- **准出标准 (DoD)**：
  - 核心模型单元测试通过；
  - 数据文件自动写入 exe 同级目录的 `data/` 文件夹下，且可整体移动拷贝；
  - DPAPI 加密与解密双向验证通过，密码在离开作用域后内存自动清零；
  - 配置文件在非破坏性升级下具备默认值反序列化能力。


---

### 阶段二：Windows 原生 RDP 宿主模块 (Phase 2)
- **目标**：打造轻量、稳定、低开销的 C++/ATL Shim，成功在子窗口中宿主 `MsRdpClient12`，打通与 Rust 的 C ABI 通信通道。
- **任务细节**：
  1. 编写 `native/host.h`、`native/active_x_host.cpp`、`native/event_sink.cpp`；
  2. 实现 ATL 容器初始化、ActiveX 控件实例化、密码与连接参数设置、销毁与资源清理；
  3. 实现 COM 事件接收器，转译 `DMsRdpClientEvents` 为强类型 C 回调；
  4. 编写 `build.rs` 利用 `cc` 自动编译 MSVC 静态库；
  5. 编写 Rust 端 `rdpm_rdp_host` 安全封装，通过通道分发事件。
- **准出标准 (DoD)**：
  - `cargo test -p rdpm_rdp_host` 编译无告警；
  - 能够成功创建 Win32 子窗口并成功连接测试 RDP 服务器；
  - 捕获并正确处理连接成功、断开与异常事件；
  - 句柄在 Drop 时干净释放，无内存泄漏与 COM 悬空指针。

---

### 阶段三：GPUI 界面与多 Tab 融合架构 (Phase 3)
- **目标**：使用 GPUI 渲染现代运维主界面，打通多 Tab 与底层 Win32 Child HWND 的像素级动态贴合。
- **任务细节**：
  1. 构建 GPUI 主窗口架构（侧边栏、主工作区、底部状态栏）；
  2. 构建左侧服务器树（支持展开收起、节点图标、双击直连、右键菜单、实时拼音过滤）；
  3. 构建顶部多 Tab 栏（Tab 标题、连接状态指示小圆点、关闭按钮、新增 Tab）；
  4. 实现 Airspace 几何贴合算法：GPUI 布局后换算屏幕物理坐标，精准下发 `SetWindowPos`；
  5. 实现 Tab 切换机制：离开 Tab 隐藏子窗口并归还 GPUI 焦点，激活 Tab 显式展示并移交键盘焦点；
  6. 引入 Anti-Jitter（防抖动）逻辑与就绪闸门（Readiness Gate）。
- **准出标准 (DoD)**：
  - 能够在 GPUI 界面中顺畅操作树节点；
  - 双击服务器能够新开 Tab 并正确嵌入展示远程桌面；
  - 多个 Tab 之间随意切换无画面残留、无穿透、无焦点卡死；
  - 窗口缩放拉伸时光标不抖动、画面不撕裂。

---

### 阶段四：会话状态机与断线自动重连 (Phase 4)
- **目标**：提供工业级稳定性的自动断线重连与异常自愈体验。
- **任务细节**：
  1. 在 `rdpm_session` 中实现有限状态机（Idle -> Connecting -> Connected -> Reconnecting -> Disconnected）；
  2. 区分用户主动关闭操作（不再重连）与网络异常中断；
  3. 实现指数退避重连算法（1s -> 2s -> 4s -> 8s -> 16s，最大重试 5 次）；
  4. 状态栏与 Tab 标题呈现动态重试计数与旋转 Spinner，支持手动“取消”与“立即重试”。
- **准出标准 (DoD)**：
  - 网络临时拔插断开时，系统自动感知并进入重连流程；
  - 网络恢复后自动无感重连回到登录桌面；
  - 用户主动点击关闭 Tab 时，立即终止重连并彻底销毁资源。

---

### 阶段五：Workspace 快照与一键平滑恢复 (Phase 5)
- **目标**：彻底超越 MultiDesk，实现复杂运维现场的一键保存与秒级环境还原。
- **任务细节**：
  1. 实现当前打开会话的状态收集器，保存所有打开的 `server_id`、Tab 排列次序与当前激活项；
  2. 建立多工作区管理（新建、切换、重命名、导出）；
  3. 支持“退出时自动保存会话”，下次启动时自动加载；
  4. 实现阶梯式并发恢复调度器（Staggered Concurrency Scheduler）：优先拉起活跃 Tab，其余会话按 250ms 间隔平滑排队拉起。
- **准出标准 (DoD)**：
  - 打开 5~8 台服务器后直接关闭 RDPM，再次启动程序时自动完整恢复打开这 8 台服务器；
  - 恢复过程平滑无卡顿，网络和 CPU 无尖峰拥塞；
  - 单台服务器连接失败不影响其余服务器的恢复。

---

### 阶段六：多屏 HiDPI 适配、综合联调与交付 (Phase 6)
- **目标**：打磨细节，完成真实环境全链路测试，产出纯绿色便携版可执行程序。
- **任务细节**：
  1. 验证 100%、125%、150%、200% 等多 DPI 屏幕拖拽切换下的分辨率自适应与 Smart Sizing；
  2. 解决 GPUI 模态弹窗与 Native HWND 的层级遮盖避让；
  3. 性能测试与资源监控（内存占用、句柄泄露、长周期稳定性）；
  4. 构建单文件绿色可执行文件 `rdpm.exe`，免安装即开即用。
- **准出标准 (DoD)**：
  - 通过所有综合联调测试用例；
  - 产出纯绿色免安装版 `rdpm.exe`，双击即可在本地自动创建 `data/` 目录运行；
  - 完整使用手册与操作文档归档。

