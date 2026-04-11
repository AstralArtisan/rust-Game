# 文档更新计划（组长例会准备）

## Context

Rust 程序设计课程第一次组长例会准备。汇报材料已生成（`docs/meeting_briefing.md`），架构问题已记录（`docs/architecture_refactor_suggestions.md`）。本计划聚焦文档更新，使其与代码现状一致。

## 执行清单

1. 更新 `docs/02_architecture.md`（mermaid 图补 6 个插件 + 3 个状态，元信息刷新）
2. 更新 `docs/03_module_design.md`（补 augment/rune/curse/skills/drops/event_room 6 个子模块）
3. 更新 `README.md` + `docs/00_index.md`（测试数 24→44，日期刷新）
4. 更新 `docs/04_api_and_data_model.md`（补数据模型）
5. 更新 `docs/07_extension_guide.md`（补扩展路径）

## 原则

- 不删除旧内容，在过时处加 `[历史快照]` 标注
- mermaid 图直接更新为当前事实
- 每个文件头部元信息统一刷新

## 验证

```bash
cargo test --quiet   # 确认 44 个测试通过
```
