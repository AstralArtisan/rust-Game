#!/bin/bash
# 在 git commit 执行后，强制提醒 Claude 运行 doc-maintenance 和 git-maintenance skill

input=$(cat)

# 用 grep 直接检测原始 JSON 字符串中是否含有 git commit（更健壮）
if echo "$input" | grep -q '"command"' && echo "$input" | grep -q 'git commit'; then
  printf '%s\n' '{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": "【工作流强制执行】刚刚完成了 git commit。在回复用户之前，你必须立即执行以下两个 skill（缺一不可）：\n1. doc-maintenance skill：更新 docs/05_iteration_history.md 和相关工程文档\n2. git-maintenance skill：检查是否需要 push，按规则决定是否推送\n这是 CLAUDE.md 中定义的强制工作流，不得跳过。"
  }
}'
fi

exit 0
