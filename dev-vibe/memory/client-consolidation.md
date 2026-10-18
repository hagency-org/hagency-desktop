# Desktop 开发已迁移至 Client

更新：2026-10-18。

- 用户要求 Desktop 合入 hagency-client，全部 Rust crate 集中在 crates/，原仓库写迁移原因后归档。
- 已通过 chrislearn/hagency-client PR #2 合并到 main：0622ce8aad384fc21efd98f1661552c5b96f979d。完整 Desktop 历史保留，主程序 crates/hagency-desktop，package/bin 仍为 rinx，根 workspace 28 个 package，共用 Cargo.lock 和本地引擎依赖。
- 本仓库 README.md、README.zh-CN.md 与 MIGRATION.md 在 cd616d95749af8560a90743c316da679de578077 已推送，说明跨仓库配对与重复维护问题、迁移目标及原Release历史下载保留。
- 实际 gh repo archive hagency-org/hagency-desktop --yes 被 GitHub ArchiveRepository 权限检查拒绝。API 核实 archived=false，当前 chrislearn 只有 push/WRITE，无 admin/maintain。用户已授权归档，但技术权限不足，需要仓库管理员完成。不可称本仓库已经归档。
- 后续代码工作请在 D:/Works/hagency-org/hagency-client 中进行；旧Release与0.5.0发布审计仍保留，不因源码合仓而视为新安装包已发布。
