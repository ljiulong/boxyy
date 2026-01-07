# 贡献指南

本项目使用 Conventional Commits，以便 release-please 自动管理版本和生成更新说明。

提交信息格式：

```
<type>: <简短描述>
```

常用类型：

- feat：新增功能（minor 版本号 +1）
- fix：修复 bug（patch 版本号 +1）
- docs：仅文档变更
- chore：维护类工作
- refactor：重构（不修复 bug，也不新增功能）
- test：新增或修改测试

破坏性变更：

- 在类型后加 `!`（例如 `feat!: ...`），或
- 在提交正文中加入 `BREAKING CHANGE:` 说明

示例：

```
feat: 增加自动更新
fix: 修复空缓存导致的报错
feat!: 移除旧配置格式
```
