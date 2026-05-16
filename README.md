# 勇闯方块城 / Block City Adventure

- 适用版本：`main` 分支
- 最后校验：2026-05-16；`cargo check` 通过，`cargo test` 86 项通过
- 源码文件数：109 个 Rust 源文件
- 关联源码：`src/main.rs`、`src/app.rs`、`src/states.rs`、`src/core/`、`src/gameplay/`、`src/coop/`、`src/pvp/`、`src/ui/`
- 实验性内容：包含。`Coop` 与 `PVP` 仍按”原型/持续完善”维护

`勇闯方块城` 是一个基于 Rust + Bevy 的 2D 俯视角动作 Roguelike 课程项目。当前仓库已经形成“单机可玩 + 合作联机原型 + PVP 原型”的总体结构，重点不在素材堆砌，而在玩法闭环、模块拆分、配置驱动和后续扩展能力。

## 技术栈
- Rust 2024
- Bevy 0.14
- `bevy_rapier2d` 0.27
- `bevy_kira_audio` 0.20
- `serde` + `ron`
- `bincode`
- `lightyear = 0.17.1`

## 快速开始
```bash
cargo run
```

发布构建：

```bash
cargo run --release
```

常用校验命令：

```bash
cargo check
cargo test
```

默认二进制名为 `block_city_adventure`。

## 基本操作
- `WASD` / 方向键：移动
- 鼠标左键 / `J`：近战攻击
- 鼠标右键：远程攻击
- `Space`：冲刺
- `1/2/3`：释放终结技（需蓄力充满）
- `E`：交互、进门、确认关键对象
- `B`：在商店房内打开商店
- `Esc`：暂停 / 返回
- `F5`：保存当前进度
- `F9`：读取存档

## 当前能力矩阵
| 领域 | 当前状态 | 说明 |
| --- | --- | --- |
| 单机主循环 | 稳定 | `MainMenu -> InGame -> Reward/Shop -> Boss -> 下一层/结算` 已闭环 |
| 战斗与成长 | 稳定 | 玩家近战/远程/冲刺、蓄力终结技、敌人 AI（6 种类型）、Boss 多阶段、商店、成就、存档 |
| 铭文/诅咒/技能 | 稳定 | 30 种铭文（4 槽位）、5 种诅咒、蓄力终结技系统、事件房系统 |
| 增强系统 | 稳定 | 被动增强池、升级回血选择、Boss 传送门奖励流程 |
| 音效与打击感 | 稳定 | 程序化音效合成（13 种 SFX）、BGM 状态机、打击暂停、屏幕闪光 |
| 配置驱动 | 稳定 | `assets/configs/*.ron` 驱动基础数值，`GameDataRegistry` 统一加载 |
| 合作联机 `Coop` | 原型 | Lightyear 主机权威模拟，Host 同进程运行 server + local client |
| 对战联机 `PVP` | 原型 | 手写 UDP 协议，独立于 `Coop` 的网络实现 |
| 本地联调 | 可用 | 通过 `LOCAL_NET_DEBUG*` 环境变量快速启动多人调试 |

## 目录概览
- `src/main.rs`：程序入口与窗口初始化
- `src/app.rs`：总插件装配点 `GamePlugin`
- `src/states.rs`：全局状态 `AppState` 与房间状态 `RoomState`
- `src/core/`：资源、输入、音频、事件、相机、存档、成就、本地联调
- `src/data/`：配置定义、加载器、全局注册表
- `src/gameplay/`：地图、玩家、战斗、敌人、奖励、商店、解谜、成长、共享规则、增强、铭文、诅咒、技能、掉落物、事件房、效果（打击暂停/屏幕闪光）
- `src/coop/`：合作模式组件、网络协议、权威运行时、联机 UI
- `src/pvp/`：PVP 网络、系统、UI
- `src/ui/`：主菜单、HUD、暂停、通知、奖励页、商店页、结算页
- `assets/`：字体、贴图、特效、配置文件
- `docs/`：完整工程交接文档

## 文档导航
- [`docs/00_index.md`](docs/00_index.md)：文档总索引、术语表、阅读顺序
- [`docs/01_build_and_run.md`](docs/01_build_and_run.md)：构建、运行、联调、配置、存档
- [`docs/02_architecture.md`](docs/02_architecture.md)：框架结构、状态机、数据流、插件树
- [`docs/03_module_design.md`](docs/03_module_design.md)：模块职责、依赖、复杂度热点
- [`docs/04_api_and_data_model.md`](docs/04_api_and_data_model.md)：内部接口契约与数据模型
- [`docs/05_iteration_history.md`](docs/05_iteration_history.md)：真实提交驱动的迭代经历
- [`docs/06_multiplayer_and_risks.md`](docs/06_multiplayer_and_risks.md)：Coop/PVP 架构差异、风险与调试
- [`docs/07_extension_guide.md`](docs/07_extension_guide.md)：后续扩展与维护指南

## 本地联调入口
仓库当前不再依赖 `local_debug/*.ps1` 脚本。真实入口为 `src/core/local_debug.rs` 的 `LocalDebugPlugin`，通过环境变量驱动：

- `LOCAL_NET_DEBUG`
- `LOCAL_NET_DEBUG_MODE`
- `LOCAL_NET_DEBUG_ROLE`
- `LOCAL_NET_DEBUG_HOST`
- `LOCAL_NET_DEBUG_SAVE_SUFFIX`

`Coop` 调试只接受裸 IPv4，不接受 `IP:端口`；端口固定为 UDP `3457`。`PVP` 使用 UDP `3456`。

## 当前质量状态
- 当前源码可通过 `cargo check`
- 当前源码可通过 `cargo test`，共 86 个单元测试（覆盖 XP 曲线、Boss 决策、奖励规则、网络协议、铭文系统、诅咒系统、蓄力系统等）
- 当前仍存在编译告警，主要集中在未使用代码和待清理的残留；这些问题已作为技术债记录在工程文档中

## 需求与历史资料
- 需求基线：`rust_game_codex_requirements.txt`
- 迭代历史：[`docs/05_iteration_history.md`](docs/05_iteration_history.md)

建议先从 [`docs/00_index.md`](docs/00_index.md) 开始阅读，再按索引顺序进入具体设计文档。

