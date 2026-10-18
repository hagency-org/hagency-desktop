# Desktop migration to hagency client

Hagency Desktop development moved to
[hagency-org/hagency-client](https://github.com/hagency-org/hagency-client) on
2026-10-18 in [PR #2](https://github.com/hagency-org/hagency-client/pull/2), merged as
[`0622ce8a`](https://github.com/hagency-org/hagency-client/commit/0622ce8aad384fc21efd98f1661552c5b96f979d).

The application and local Agent engine evolve together. A separate repository
required matching their Git revisions for every shared identity, permission,
Project or runtime change. The unified repository can update both sides atomically,
validate them together, and maintain one Cargo dependency lock and build configuration.
It also provides a single home for platform CI and installer tooling.

All 28 Rust packages now live under `crates/`. The native application is in
[`crates/hagency-desktop`](https://github.com/hagency-org/hagency-client/tree/main/crates/hagency-desktop),
with the Cargo package and executable name `rinx` retained. The engine is a local
path dependency. Read the [workspace guide](https://github.com/hagency-org/hagency-client/blob/main/docs/workspace.md)
for the new layout and commands, including the Windows D: build-cache wrapper.

The complete Desktop history through `52d88b2020a322d9c1883b412e74968c1a60ce01`
is preserved as an ancestor in the new repository. Original Apache/MIT licenses
and third-party notices remain with the code. Existing releases stay available
here so historical download links continue to work. Source consolidation does
not publish new installers or resolve the existing 0.5.0 release assessment.

Use the new repository for future code, issues and pull requests. This repository
is being retired from active development and retained as a historical archive.

## 中文说明

Desktop 和本地 Agent 引擎的界面与协议需要共同演进，分仓维护会导致提交配对、
依赖版本及构建配置的重复维护。现已保留历史并合入 hagency-client，所有 crate
集中到根 crates/，后续开发和问题反馈统一转移到新仓库。

原仓库保留已有源码历史、许可及 Release 下载，退出日常开发维护。应用数据格式
和 rinx 二进制名不因本次目录整理而修改；合仓不代表发布新的安装包。
