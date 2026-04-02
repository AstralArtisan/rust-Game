# 工程交接文档索引

- 适用版本：当前工作树（HEAD `aa90cf3c`，tag `saved-version-20260330-161713`）
- 最后校验：2026-03-31；`cargo check` 通过，`cargo test` 24 项通过
- 关联源码：`README.md`、`src/`、`assets/configs/`、`docs/`
- 实验性内容：包含。本文档会显式区分稳定模块、联机原型和历史审计材料

本文档集面向工程交接与后续维护。阅读目标不是“快速宣传项目”，而是帮助维护者在最短时间内搞清楚以下问题：

1. 程序如何启动，状态如何流转。
2. 单机、Coop、PVP 为什么共享部分玩法系统又保留不同网络实现。
3. 哪些目录是稳定骨架，哪些文件是复杂度热点。
4. 当前哪些文档是历史资料，哪些是当前事实基线。

## 推荐阅读顺序
1. [`README.md`](../README.md)：项目概览、能力矩阵、文档导航
2. [`01_build_and_run.md`](01_build_and_run.md)：构建、运行、联调、配置、存档
3. [`02_architecture.md`](02_architecture.md)：系统总装配、状态机、ECS 分层、数据流
4. [`03_module_design.md`](03_module_design.md)：各模块职责、依赖、复杂度热点
5. [`04_api_and_data_model.md`](04_api_and_data_model.md)：内部接口契约与数据模型
6. [`06_multiplayer_and_risks.md`](06_multiplayer_and_risks.md)：联机原型专项说明
7. [`07_extension_guide.md`](07_extension_guide.md)：扩展和维护落地指南
8. [`05_iteration_history.md`](05_iteration_history.md)：基于真实提交的演进脉络

## 文档地图
| 文档 | 角色 | 稳定度标签 | 适合谁先看 |
| --- | --- | --- | --- |
| [`README.md`](../README.md) | 仓库入口页 | 当前事实 | 所有人 |
| [`01_build_and_run.md`](01_build_and_run.md) | 构建、运行、联调手册 | 当前事实 | 首次接手者 |
| [`02_architecture.md`](02_architecture.md) | 总体架构、状态机、数据流 | 当前事实 | 架构理解 |
| [`03_module_design.md`](03_module_design.md) | 模块职责与依赖说明 | 当前事实 | 开发维护 |
| [`04_api_and_data_model.md`](04_api_and_data_model.md) | 内部接口契约、数据模型 | 当前事实 | 改功能前必须看 |
| [`05_iteration_history.md`](05_iteration_history.md) | 真实演化历史 | 历史事实 | 答辩、追踪设计意图 |
| [`06_multiplayer_and_risks.md`](06_multiplayer_and_risks.md) | 联机原型说明与技术债 | 当前事实 + 风险 | 联机维护者 |
| [`07_extension_guide.md`](07_extension_guide.md) | 新增内容时的改动路径 | 当前事实 | 功能扩展者 |
| [`coop_network_audit.md`](coop_network_audit.md) | 早期 Coop 审计记录 | 历史审计 | 仅作背景参考 |
| [`project_overview_and_coop_review.md`](project_overview_and_coop_review.md) | 中期评审快照 | 历史评审 | 对照旧结论时参考 |

## 术语表
| 术语 | 含义 |
| --- | --- |
| `AppState` | 全局游戏状态机，覆盖 Loading、单机、Coop、PVP、结算等状态 |
| `RoomState` | 当前房间推进状态，控制房门、刷怪、清房和 Boss 战阶段 |
| `GamePlugin` | `src/app.rs` 中的顶层插件装配器，负责把整个工程拼起来 |
| `GameDataRegistry` | 从 `assets/configs/*.ron` 加载出的统一配置注册表 |
| `InGameEntity` | 单机/房间内世界实体通用标记，用于清理当前局内对象 |
| `Replicated` | Lightyear 的复制体标记；Coop 中 Host 会同时看到权威实体和复制实体 |
| Host Authority | 主机权威模拟。Coop 中只有 Host 负责真正推进玩法逻辑 |
| `session_core` | 抽离出的共享规则层，封装单机/Coop 共享的奖励、商店、死亡决策等 |
| `LocalDebugPlugin` | 通过环境变量快速进入 Coop/PVP 调试流程的本地联调入口 |

## 文档标签约定
- `稳定`：单机主循环、配置加载、存档、成就、基础 UI 等已形成稳定骨架。
- `原型`：Coop、PVP、本地多人联调、部分联机 UI 和协议仍处于持续完善阶段。
- `历史审计`：只反映某个时间点的观察结果，不应直接替代当前源码事实。

## 源码基线
- 当前主工程名：`block_city_adventure`
- `Cargo.toml` 关键依赖：`bevy 0.14`、`bevy_rapier2d 0.27`、`bevy_kira_audio 0.20`、`lightyear 0.17.1`
- 当前源码文件数量：92 个 Rust 源文件
- 当前复杂度热点：`src/coop/runtime.rs`、`src/coop/ui.rs`、`src/gameplay/enemy/systems.rs`、`src/gameplay/session_core/mod.rs`、`src/ui/hud.rs`

## 当前质量结论
- `cargo check` 可通过，但仍存在大量告警
- `cargo test` 当前通过 33 个单元测试
- 现有测试主要集中在 `session_core`、`coop::net`、`coop::runtime` 与部分玩家行为约束
- 多人文档已从旧的“0 tests”阶段演进，阅读旧文档时必须注意时间背景

## 使用方式
- 如果要快速跑起来：先看 [`01_build_and_run.md`](01_build_and_run.md)
- 如果要改单机玩法：先看 [`02_architecture.md`](02_architecture.md) 和 [`03_module_design.md`](03_module_design.md)
- 如果要改联机：先看 [`06_multiplayer_and_risks.md`](06_multiplayer_and_risks.md)
- 如果要新增功能：直接对照 [`07_extension_guide.md`](07_extension_guide.md) 的改动清单
