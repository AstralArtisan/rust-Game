# 测试模式（临时设施）

> ⚠️ 本文件为**临时测试设施**说明，非正式功能。测试结束后整套机制连同本文档一并删除。
> 不要在此机制上继续扩展或建立依赖。

## 用途与行为

为方便组员测试游戏内容，主菜单新增「测试模式」入口。其与「开始冒险」唯一的区别：

- 玩家死亡（血量归零）后**不进入 GameOver**，而是**原地满血复活**；
- 复活瞬间附带**约 2 秒无敌**，避免在敌群中倒下后被反复秒杀的「死亡循环」（复用游戏已有的 `InvincibilityTimer`）；
- **仅单机生效**（主菜单「开始冒险 / 测试模式」走的 `AppState::InGame` 单机路径）。联机合作 Coop 死亡走 `src/coop/runtime.rs` 独立逻辑，未做改动，不受影响。

从主菜单点「开始冒险」进入的正常对局**不受影响**：死亡仍正常进入 GameOver。

## 实现原理

`evaluate_death(SessionMode::Solo, 0)` 在单机下恒返回 `GameOver`，且 `player_death_system` 不检查血量，因此提前回血无法阻止 GameOver。拦截点放在 `player_death_system` 内部：检测到玩家 `DeathEvent` 且 `TestMode(true)` 时，直接把 `Health.current` 拉满、刷新 `InvincibilityTimer`，并 `return` 跳过 GameOver 状态切换。

## 改动点清单

| # | 文件 | 改动内容 |
|---|------|----------|
| 1 | `src/core/test_mode.rs` | **新增**。定义 `TestMode(pub bool)` 资源。 |
| 2 | `src/core/mod.rs` | 新增 `pub mod test_mode;`。 |
| 3 | `src/app.rs` | `GamePlugin::build` 中新增 `.init_resource::<crate::core::test_mode::TestMode>()`。 |
| 4 | `src/ui/menu.rs` | 新增 `use crate::core::test_mode::TestMode;`；`MainMenuButton` 增 `TestMode` 变体；`setup_main_menu` 增「测试模式」按钮；`menu_button_system` 中 `SinglePlayer` 分支追加 `insert_resource(TestMode(false))`，新增 `TestMode` 分支 `insert_resource(TestMode(true))`。 |
| 5 | `src/gameplay/player/systems.rs` | 新增 `use crate::core::test_mode::TestMode;`；改写 `player_death_system`：查询改为可变 `(Entity, &mut Health, &mut InvincibilityTimer)`，新增 `test_mode: Res<TestMode>`，命中玩家死亡且测试模式开启时满血复活 + 2 秒无敌并跳过 GameOver。 |
| 6 | `docs/test_mode_temp.md` | **新增**（本文件）。 |

## 逐步删除指引（测试结束后）

1. **删除** `src/core/test_mode.rs` 整个文件。
2. `src/core/mod.rs`：删除 `pub mod test_mode;` 这一行。
3. `src/app.rs`：删除 `.init_resource::<crate::core::test_mode::TestMode>()` 这一行。
4. `src/ui/menu.rs`：
   - 删除 `use crate::core::test_mode::TestMode;`；
   - `MainMenuButton` 枚举删除 `TestMode,` 变体；
   - `setup_main_menu` 删除 `spawn_menu_button(menu, &assets, "测试模式", MainMenuButton::TestMode);` 这一行；
   - `menu_button_system`：删除整个 `MainMenuButton::TestMode => { ... }` 分支；从 `SinglePlayer` 分支删除 `commands.insert_resource(TestMode(false));` 这一行。
5. `src/gameplay/player/systems.rs`：
   - 删除 `use crate::core::test_mode::TestMode;`；
   - 把 `player_death_system` 还原为：参数 `player_q: Query<Entity, (With<Player>, Without<Replicated>)>`、去掉 `test_mode` 参数，函数体恢复为「命中玩家死亡 → `evaluate_death(Solo,0)==GameOver` 则 `next_state.set(GamePhase::GameOver)`」的原始两段式逻辑。
6. **删除** `docs/test_mode_temp.md`（本文件）。
7. 运行 `cargo fmt`、`cargo clippy`、`cargo check`、`cargo test` 确认干净。

完成以上步骤后，无任何其他文件依赖该机制。
