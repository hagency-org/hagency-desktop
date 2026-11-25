# Desktop 0.5.0 发布

更新：2026-11-25。

## 2026-11-25 rc.19 发布

rc.18 之后 main 上合入了五组产品改动；用户同意从 main 出 rc.19；由 Claude 执行。

- [v0.5.0-rc.19](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.19) 于 2026-11-25 04:28:32 UTC 公开为 prerelease。标签和两个安装包都由 Desktop `a1f2ffb16ec174fbbbb8a9c2e6b4e2781df03c78` 构建，即发布提交（PR #232 的合并提交）本身。rc.18 之后合入的产品改动：PR #218（不再给无法应答的 Agent 建私聊；没有 Agent 工作的房间不显示权限告警）、#171（Matrix 身份检查短暂失败时保留原来的 Codex 登录）、#229（把 #60、#61、#66、#68、#70、#72、#73 即 R07–R13 搬到当时的 main 上：分享附件的临时文件私有化、权限状态读取失败可恢复、房间和 Space 的创建在应答丢失后可找回、结果未知的运行改为只读复核、设备列表与确认后的移除、音视频在系统播放器中打开、默认关闭的系统通知、Space 列表可见的滚动条）、#230（把 #127 和 #220 搬到 main 上：房间、邀请和选择器的修正，更多界面文字的翻译，macOS 以外不带容器的共享 Claude 以明确的错误码拒绝）和 #231（托管电脑向服务器申请两分钟的授权时长和同样长的租约）；另有只改文档的 PR #219。`crates/` 下有改动（15 个文件），托管服务与 rc.18 不同；托管数据库的表没有变化。Makepad 与 rc.18 相同，为 `8f788cbb788f287f4e684de913e83fbbb70df701`。
- 发布运行 [45535856540](https://github.com/hagency-org/hagency-desktop/actions/runs/45535856540)（publish=true、macos-arm64-windows）在 main 上第一次尝试全部通过。发布 PR 只改版本号和发布说明，合并时没有等它自己的检查：合并前的 main 的 14 项检查全部通过，发布 PR 分支的内容与发布提交完全相同。发布 PR 自己那一轮在项目自己的 Mac 上的两项检查在合并后被取消，把 Mac 让给发布运行；它们与发布提交在 main 上的检查内容相同。发布提交在 main 上的检查全部通过；其中一个 Linux 构建作业（ubuntu-22.04-arm）第一次在克隆 git 依赖时遇到网络错误而失败，重跑后通过，与代码无关。
- 用时：从触发到公开 97 分钟。回归门禁 17.7 分钟，桌面测试 macOS 6.2 分钟、Windows 26.4 分钟，macOS 安装包在项目自己的发布机上 11.3 分钟，Windows 安装包 92.6 分钟，是整个发布的用时。回归门禁和 macOS 桌面测试先在队列里等了大约 25 分钟，等项目自己的 Mac 跑完发布 PR 的检查；这段等待没有延长总用时。两个平台的随包内核都复用了此前 main 上发布运行留存的内核（macOS 来自 44649851440，Windows 来自 44627576533），没有从源码构建。
- 只有两个安装包：`hagency-desktop-macos-arm64.dmg`（176,492,436 字节）和 `hagency-desktop-windows-amd64.exe`（126,010,001 字节）。公开附件已重新下载，`SHA256SUMS.txt` 和两个平台各自的校验文件都校验通过，两份构建报告都写明上述提交和版本 0.5.0-rc.19。
- 在构建机以外的一台 Mac 上，用空 profile 和隐藏窗口，通过 `tools/release-desktop-smoke.py` 运行了公开的 macOS 安装包：第一次运行通过，登录页和包内标志已绘制，随包内核为 `octos 2.0.3-rc.13 (fe08d8e)`，没有资源错误。Windows 安装包没有在实机上运行。
- 改动合并前的验证：#218 的检查全部通过后合并；#171 先合到当时的 main 上跑完检查再合并。R07–R13 和 #127、#220 是按 11 月 11 日前后的源码写的，无法直接合并，由发布者逐个搬到当时的 main 上，原作者保留；搬运时两边不一致的地方所做的取舍写在 #229、#230 和它们引用的 PR 里。#229、#230、#231 各自的 14 项检查全部通过后合并，其中 #229 和 #231 没有重跑才通过的测试。#231 的服务端一侧是 hagency-server #95，它的检查包含一个用真实数据库验证续期请求协商结果的测试。
- 发布前没有由发布者在运行的应用里试过 rc.19 的任何新功能。原作者在各自的 PR 里报告：#218 用 release 构建和两个 profile 验证过；R12 的音视频播放和系统通知于 11 月 12 日在 macOS 上按当时的源码验证过；R13 的滚动条有无界面的绘制测试。发布运行开始后，一台 Mac 的托管服务和两个已登录的窗口（JIT 运行器下的开发构建）改为运行发布提交的源码，启动、托管和聊天界面正常，没有在它们上面打开新页面。托管服务那次冷启动后，五个本地 Agent 中有一个没有恢复托管，日志里没有任何告警；把托管停止再恢复一次后五个全部恢复。启动时某个 Agent 恢复失败只计入总数、不写日志也不重试，这是 rc.18 就有的行为，尚未修复。rc.19 的任何改动都没有在 Windows 上手动运行。
- 这次发布没有部署服务端。#231 的两分钟授权时长需要服务器带有 hagency-server #95（已在服务端 main 上，还没有部署到任何服务器），并在主机配置里把 `authorization_window_ms` 设到默认值 30000 以上（最大 120000）；缺少任何一项时服务器仍给 30 秒，Desktop 的行为与 rc.18 相同。共享资源池执行自己的 30 秒时长没有改变。rc.19 的其他改动目前没有发现需要服务端改动。

## 2026-11-24 rc.18 发布

rc.17 之后 main 上合入了四个产品改动；用户同意从 main 出 rc.18；由 Claude 执行。

- [v0.5.0-rc.18](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.18) 于 2026-11-24 09:15:17 UTC 公开为 prerelease。标签和两个安装包都由 Desktop `6bc3ff0c9df9196fc736b5916b3efe9efafe55f0` 构建，即发布提交（PR #217 的合并提交）本身。rc.17 之后合入的产品改动：PR #203（共享 Codex Agent 的第一条消息在托管电脑接手后不再那么容易过期：先读 Codex 账号再申请启动许可，取工作期间续期活动）、#213（Space 打开之后才链接进来的房间和子 Space 会显示出来）、#214（设备还需要完成设置时，编码工具列表、资源池列表和创建资源池的表单会说明去哪里完成）和 #215（由另一台 homeserver 创建的 Space 显示它登记的类型）；另有只改文档的 PR #211。`crates/` 下有改动（8 个文件，都来自 #203），托管服务与 rc.17 不同；托管数据库的表没有变化。Makepad 与 rc.17 相同，为 `8f788cbb788f287f4e684de913e83fbbb70df701`。
- 发布运行 [45351942405](https://github.com/hagency-org/hagency-desktop/actions/runs/45351942405)（publish=true、macos-arm64-windows）在 main 上第一次尝试全部通过。发布 PR 只改版本号和发布说明，合并时没有等它自己的检查：合并前的 main 在三个工作流里全部通过，发布 PR 分支的内容与发布提交完全相同，它的 14 项检查随后也全部通过。合并前的 main 上有一个测试（`inline_factory` 的 `native_reattach_honours_the_fence`）第一次失败、自动重跑后通过；这是已知不稳定的测试，四个改动都没有碰它的代码，发布运行里它一次通过。
- 用时：从触发到公开 78 分钟。回归门禁 21.1 分钟，桌面测试 macOS 5.0 分钟、Windows 17.5 分钟，macOS 安装包在项目自己的发布机上 11.7 分钟，Windows 安装包 75.0 分钟，是整个发布的用时。两个平台的随包内核都复用了此前 main 上发布运行留存的内核（macOS 来自 44649851440，Windows 来自 44627576533），没有从源码构建。
- 只有两个安装包：`hagency-desktop-macos-arm64.dmg`（176,151,467 字节）和 `hagency-desktop-windows-amd64.exe`（125,469,290 字节）。公开附件已重新下载，`SHA256SUMS.txt` 和两个平台各自的校验文件都校验通过，两份构建报告都写明上述提交和版本 0.5.0-rc.18。
- 在构建机以外的一台 Mac 上，用空 profile 和隐藏窗口，通过 `tools/release-desktop-smoke.py` 运行了公开的 macOS 安装包：第一次运行通过，登录页和包内标志已绘制，随包内核为 `octos 2.0.3-rc.13 (fe08d8e)`，没有资源错误。Windows 安装包没有在实机上运行。
- 四个改动合并前的验证：它们都是别人的 PR，合并前各自的检查全部通过。#203 合并时 main 已经前进，所以先把它合到当时的 main 上，用一个不合并的草稿 PR（#212）跑完全部检查再合并。#213、#214、#215 一起合到 main 上，用草稿 PR #216 跑完 14 项检查，并在 JIT 运行器下把这个组合启动到登录页；三个 PR 合并后 main 的内容与验证过的组合完全相同。
- 发布前没有由发布者在真实账号上手动运行 rc.18 的任何改动。PR 的作者在 PR 里报告：带 #203 改动的托管 Mac 上，原先失败的两个会话各被回答一次；Space 类型在两台本地测试服务器之间被两个账号都正确读到。发布运行开始后，一台 Mac 的托管服务和两个已登录的窗口（JIT 运行器下的开发构建）改为运行发布提交的源码，启动和托管正常；没有在它们上面专门验证这四项改动。rc.18 的任何改动都没有在 Windows 上运行。
- 这次发布没有部署服务端。#215 的 Space 类型需要成员自己的服务器和创建该 Space 的账号所在的服务器都带有 hagency-server #87（已在服务端 main 上，还没有部署到任何服务器）。在没有它的服务器上，由另一台 homeserver 创建的 Space 显示“Space type unavailable”，Desktop 不在其中创建或添加房间，也不为它登记项目；在自己服务器上创建的 Space 不受影响。rc.18 的其他改动不需要服务端改动。

## 2026-11-22 rc.17 发布

rc.16 发布当天发现它在托管 Agent 的电脑上会拒收有效的租约续期；用户同意按建议从 main 出 rc.17；由 Claude 执行。

- [v0.5.0-rc.17](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.17) 于 2026-11-22 20:45:51 UTC 公开为 prerelease。标签和两个安装包都由 Desktop `ec6c86de05645f8b0ac77ac8af36c9d77eb330b1` 构建，即发布提交（PR #210 的合并提交）本身。rc.16 之后合入的产品改动：PR #208（托管电脑不再拒收 Agent 租约的有效续期，是出这一版的原因）、#204（邀请页显示房间里的人）、#206（创建 Claude 资源池时可以选择容器；容器镜像里 git、用户查询和提交作者可用）和 #207（聊天滚动时少做工作，隐藏的小程序标签页不再收到帧）；另有只改测试的 PR #209 和只改文档的 PR #205。`crates/` 下有改动（21 个文件），托管服务与 rc.16 不同；托管数据库的表没有变化。Makepad 与 rc.16 相同，为 `8f788cbb788f287f4e684de913e83fbbb70df701`。
- 发布运行 [45059295376](https://github.com/hagency-org/hagency-desktop/actions/runs/45059295376)（publish=true、macos-arm64-windows）在 main 上第一次尝试全部通过。发布 PR 只改版本号和发布说明，合并时没有等它自己的检查：合并前的 main 在三个工作流里全部通过，发布 PR 分支的内容与发布提交完全相同，它的 14 项检查随后也全部通过。
- 用时：从触发到公开 95 分钟。回归门禁 17.2 分钟，桌面测试 macOS 6.2 分钟、Windows 24.9 分钟，macOS 安装包在项目自己的发布机上 10.9 分钟，Windows 安装包 91.8 分钟，是整个发布的用时。两个平台的随包内核都复用了此前 main 上发布运行留存的内核（macOS 来自 44649851440，Windows 来自 44627576533），没有从源码构建。
- 只有两个安装包：`hagency-desktop-macos-arm64.dmg`（176,006,397 字节）和 `hagency-desktop-windows-amd64.exe`（125,428,021 字节）。公开附件已重新下载，`SHA256SUMS.txt` 和两个平台各自的校验文件都校验通过，两份构建报告都写明上述提交和版本 0.5.0-rc.17。
- 在构建机以外的一台 Mac 上，用空 profile 和隐藏窗口，通过 `tools/release-desktop-smoke.py` 运行了公开的 macOS 安装包：第一次运行通过，登录页和包内标志已绘制，随包内核为 `octos 2.0.3-rc.13 (fe08d8e)`，没有资源错误。Windows 安装包没有在实机上运行。
- rc.16 的缺陷：rc.16 的托管电脑把服务器给的有效租约续期当作 `execution_scope_mismatch` 拒收，每次持续几秒，直到下一次设备授权。原因是 PR #181 让 Desktop 把自己的授权看作比服务器写明的时间早一点结束，而租约校验仍拿续期后的租约和这个较早的时间比；服务器把租约截到授权结束的时间，所以有效的续期被拒。在一台 Mac 上 12 分钟有 613 次拒收，那段时间里没有租约过期；拒收若持续到租约到期，正在运行的回合会被中断。托管 Agent 的电脑应从 rc.16 更新到 rc.17；被 rc.16 打开过托管登记的 Mac 不能退回 rc.15（见 rc.16 一条）。
- 发布前在真实账号上手动运行过的内容：一台 Mac 的托管服务（JIT 运行器下的开发构建，持有 5 个租约）先运行 rc.16 的源码，再运行带修正的源码，修正后两个多小时没有一次拒收；在已登录的窗口里，用创建页上的容器开关创建了一个未发布的 Claude 资源池。发布运行开始后，同一台 Mac 的托管服务改为运行发布提交的源码，到写这条记录时 1 小时 38 分钟里没有一次拒收，也没有租约过期。没有实跑的内容写在发布说明末尾 rc.17 一段里：这样创建的资源池里 Agent 的运行、资源池等待容器的状态、点名房间成员的邀请（还没有部署会这样做的服务器）和滚动改动；rc.17 的任何改动都没有在 Windows 上运行。
- 这次发布没有部署服务端。邀请页的成员列表需要 hagency-server #86，并且服务器的 `palpo.toml` 里 `invite_state_member_limit` 大于 0；否则邀请页最多显示人数。rc.17 的其他改动不需要服务端改动。

## 2026-11-22 rc.16 发布

用户要求出 rc；由 Claude 执行。

- [v0.5.0-rc.16](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.16) 于 2026-11-22 07:19:46 UTC 公开为 prerelease。标签和两个安装包都由 Desktop `3d6e57a32d1315fe7c1be05f70fb6941e29e1f1b` 构建，即发布提交（PR #202 的合并提交）本身。rc.15 之后合入的产品改动：PR #182（已存在的 Space 可以补完设置）、#183（恢复已保存账号时重试服务器发现）、#184（macOS 托管登记改按卷标识绑定）、#185 和 #198（确认资源池、首次发布资源池时取当前的本地证明）、#186 和 #188（共享 Agent 启动前先检查 Codex provider，托管服务按自己的节奏查找新请求；#188 经 PR #196 带入）、#128（接受房间版本 12 的房间 ID）、#181（设备时钟偏慢时的 owner 授权校验）、#179（我的 Agent 页的入门说明，经 PR #196 带入）、#199（服务器写入繁忙时重发共享 Agent 已完成的回答）、#187（读取原始话题的运行状态不受房间条数限制）和 #189（额度不足的提示和从卡片到原始资源池请求的链接），两者经 PR #200 带入、#197（程序文件改名为 `hagency`）和 #201（改名后在启动时重新登记托管服务）；另有只动依赖的 PR #133 和只改测试的 PR #124。`crates/` 下有改动（63 个文件），托管服务与 rc.15 不同；托管数据库的表没有变化。Makepad 与 rc.15 相同，为 `8f788cbb788f287f4e684de913e83fbbb70df701`。
- 发布运行 [44991675575](https://github.com/hagency-org/hagency-desktop/actions/runs/44991675575)（publish=true、macos-arm64-windows）在 main 上第一次尝试全部通过。发布 PR 分支的内容与发布提交完全相同，它的检查跑在同一份内容上。
- 用时：从触发到公开 81 分钟。回归门禁 17.4 分钟，桌面测试 macOS 4.8 分钟、Windows 21.6 分钟，macOS 安装包在项目自己的发布机上 11.2 分钟，Windows 安装包 79.4 分钟，是整个发布的用时。两个平台的随包内核都复用了此前 main 上发布运行留存的内核（macOS 来自 44649851440，Windows 来自 44627576533），没有从源码构建。
- 只有两个安装包：`hagency-desktop-macos-arm64.dmg`（176,057,650 字节）和 `hagency-desktop-windows-amd64.exe`（125,659,372 字节）。公开附件已重新下载，`SHA256SUMS.txt` 和两个平台各自的校验文件都校验通过，两份构建报告都写明上述提交和版本 0.5.0-rc.16。
- 在构建机以外的一台 Mac 上，用空 profile 和隐藏窗口，通过 `tools/release-desktop-smoke.py` 运行了公开的 macOS 安装包：第一次运行通过，登录页和包内标志已绘制，随包内核为 `octos 2.0.3-rc.13 (fe08d8e)`，没有资源错误。Windows 安装包没有在实机上运行。
- 发布前没有在真实账号上手动运行 rc.16 的任何改动；发布说明末尾 rc.16 一段列出了没有实跑的内容，其中包括：在托管 Agent 的 Mac 上更新已安装的应用（托管服务跟随新程序名只由对其判断的测试覆盖），以及之后退回 rc.15。
- 退回的限制：rc.16 第一次打开已有的托管登记时，把它的绑定改写为按卷标识的形式，rc.15 及更早的版本不接受这种绑定，托管在那里打不开。托管 Agent 的 Mac 升到 rc.16 后要留在 rc.16 或更新的版本。
- 这次发布没有部署服务端。rc.16 的额度提示和从被拦卡片到资源池请求的链接需要 hagency-server #73 和 #75；服务器没有它们时，Desktop 显示 rc.15 的内容。

## 2026-11-21 rc.15 发布

用户要求合并 PR #194、#193，随后要求出 rc，并要求带上 PR #192；由 Claude 执行。

- [v0.5.0-rc.15](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.15) 于 2026-11-21 05:59:16 UTC 公开为 prerelease。标签和两个安装包都由 Desktop `9aaab2ebe7f4a5b4ee4c2e2ee8cb19e87bbf90f5` 构建，即发布提交（PR #195 的合并提交）本身：rc.14 之后合入了产品改动 PR #190（朋友圈在聊天窗格内打开）、#191（成员列表每次从服务器重读）、#194（Markdown、HTML、PDF 附件在阅读器中预览，PDF 阅读页可选字、查找、缩放，回答下方的按钮改为打开阅读器标签页）、#193（房间信息改为单页，成员宫格加减，房间改名，房间内昵称，聊天记录不再盖住房间列表）和 #192（已完成的运行不再留卡片），以及只改 CI 和文档的 PR #180。`crates/` 下没有改动，托管服务和托管数据库与 rc.14 相同。Makepad 与 rc.14 相同，为 `8f788cbb788f287f4e684de913e83fbbb70df701`。
- 发布运行 [44866091218](https://github.com/hagency-org/hagency-desktop/actions/runs/44866091218)（publish=true、macos-arm64-windows）在 main 上第一次尝试全部通过。发布 PR 的分支在合并前并入了当时的 main，它的检查跑在与发布提交完全相同的内容上。
- 用时：从触发到公开 93 分钟。回归门禁 19 分钟，桌面测试 macOS 5.5 分钟、Windows 25 分钟，macOS 安装包在项目自己的发布机上 11.5 分钟，Windows 安装包 92 分钟，是整个发布的用时。两个平台的随包内核都复用了此前 main 上发布运行留存的内核（macOS 来自 44649851440，Windows 来自 44627576533），没有从源码构建。
- 只有两个安装包：`hagency-desktop-macos-arm64.dmg`（174,770,396 字节）和 `hagency-desktop-windows-amd64.exe`（125,104,864 字节），比 rc.14 各大约 5 MB 和 2.6 MB，主要是随包的 PDF.js。公开附件已重新下载，`SHA256SUMS.txt` 和两个平台各自的校验文件都校验通过，两份构建报告都写明上述提交和版本 0.5.0-rc.15。
- 在构建机以外的一台 Mac 上，用空 profile 和隐藏窗口，通过 `tools/release-desktop-smoke.py` 运行了公开的 macOS 安装包：第一次运行通过，登录页和包内标志已绘制，随包内核为 `octos 2.0.3-rc.13 (fe08d8e)`，没有资源错误。Windows 安装包没有在实机上运行。
- 发布前在真实账号上手动运行过的内容和没有运行过的内容，写在发布说明末尾 rc.15 一段里；rc.15 的任何改动都没有在 Windows 上运行。
- 这次发布没有部署服务端，也不需要服务端改动。

## 2026-11-20 rc.14 发布

用户要求发布 rc.14，并缩短发布流程；由 Claude 执行。

- [v0.5.0-rc.14](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.14) 于 2026-11-20 00:31:54 UTC 公开为 prerelease。标签和两个安装包都由 Desktop `71d6e9adba17b15dcb33c15732320c4286c45735` 构建：rc.13 之后合入了产品改动 PR #157、#160、#161、#162、#165、#167、#170、#172 和 #168（经 PR #173 合入，一台电脑为多个账号托管 Agent），版本和发布说明 PR #177，以及只改 CI、测试和打包工具的 PR #163、#169、#175、#176、#178。Makepad 与 rc.13 相同，为 `8f788cbb788f287f4e684de913e83fbbb70df701`。
- 第一次发布运行 [44608655460](https://github.com/hagency-org/hagency-desktop/actions/runs/44608655460) 在发布提交 `7b737c5b` 上触发，回归门禁等了 15 分钟没有等到 GitHub 的 Mac，被 GitHub 取消，因此没有发布。之后把缩短发布流程的改动（PR #178：门禁和 Apple Silicon 桌面测试改在项目自己的 Mac 上运行，桌面测试与打包并行，随包 Octos 内核按固定版本留存复用）提前合入，先在分支上用一次不发布的运行 [44611946100](https://github.com/hagency-org/hagency-desktop/actions/runs/44611946100) 完整走通，再从 main 触发 [44627576533](https://github.com/hagency-org/hagency-desktop/actions/runs/44627576533)（publish=true、macos-arm64-windows），第一次尝试全部通过并发布。因此标签不在发布提交上，而在其后只多了 PR #178 的合并提交上；两者的产品代码相同。
- 用时：从触发到公开 132 分钟。门禁 17 分钟（此前在 GitHub 的 Mac 上 82 至 96 分钟），macOS 安装包 113 分钟，Windows 安装包 131 分钟。内核复用在这次运行中没有生效：这是 main 上第一次留存内核的运行，两个平台的内核都从源码构建并留存。
- 只有两个安装包：`hagency-desktop-macos-arm64.dmg`（169,898,512 字节）和 `hagency-desktop-windows-amd64.exe`（122,534,555 字节）。公开附件已重新下载，`SHA256SUMS.txt` 和两个平台各自的校验文件都校验通过，两份构建报告都写明上述提交和版本 0.5.0-rc.14，并新增了内核一项（固定版本、校验和、构建它的运行）。
- 在构建机以外的一台 Mac 上，用空 profile 和隐藏窗口，通过 `tools/release-desktop-smoke.py` 运行了公开的 macOS 安装包：第一次运行通过，随包内核为 `octos 2.0.3-rc.13 (fe08d8e)`，没有资源错误。Windows 安装包没有在实机上运行。
- 发布前没有在真实账号上手动运行 rc.14 的任何新功能，包括 PR #168 对托管服务的改动；发布说明末尾列出了没有实跑的内容。
- 这次发布没有部署服务端。

## 2026-11-19 rc.13 发布

用户要求基于最新合入的 PR 发布新的 RC；由 Claude 执行。

- [v0.5.0-rc.13](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.13) 于 2026-11-19 04:40:58 UTC 公开为 prerelease。标签和两个安装包都由 Desktop `5d9933a9d04577d0637723e35f369abe5f1c6e61` 构建：rc.12 之后合入了按资源池选择让 Claude 池 Agent 在容器中运行的 PR #156（默认关闭）、托管管理库关闭顺序的一行修正（在 PR #159 中），以及版本和发布说明 PR #164；其余是只改 CI 的 PR #154、#158、#159。Makepad 与 rc.12 相同，为 `8f788cbb788f287f4e684de913e83fbbb70df701`。
- [release.yml run 44445855545](https://github.com/hagency-org/hagency-desktop/actions/runs/44445855545) 使用 publish=true、macos-arm64-windows，第一次尝试中回归门禁、两平台安装包检查和发布全部通过，从触发到公开 2 小时 02 分。
- 只有两个安装包：`hagency-desktop-macos-arm64.dmg`（169,884,184 字节）和 `hagency-desktop-windows-amd64.exe`（122,505,475 字节）。公开附件已重新下载，`SHA256SUMS.txt` 和两个平台各自的校验文件都校验通过，两份构建报告都写明上述提交和版本 0.5.0-rc.13。
- 在构建机以外的一台 Mac 上，用空 profile 和隐藏窗口，通过 `tools/release-desktop-smoke.py` 运行了公开的 macOS 安装包：第一次运行通过，随包内核为 `octos 2.0.3-rc.13 (fe08d8e)`，没有资源错误。Windows 安装包没有在实机上运行。
- 容器模式在 PR #156 合入前由其作者在一台 Mac 上对两个版本各跑通过一次；发布执行者本人没有运行容器模式，发布说明中这一节依据 PR 的说明写成，并由 PR 作者对照合入的代码核对过。
- 这次发布没有部署服务端。

## 2026-11-18 rc.12 发布

用户要求合并当时已通过检查的五个 PR 并通过 CI 发布；由 Claude 执行。

- [v0.5.0-rc.12](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.12) 于 2026-11-18 21:54:22 UTC 公开为 prerelease。标签和两个安装包都由 Desktop `94a0cfa4f25b123807a618c83b539da9587b9cb8` 构建：rc.11 之后合入了 Agent 标记对齐和账号设置页排版 PR #146、日志阻塞时发送不受影响并不再把消息正文写入诊断日志 PR #148、按所显示的内容校验 TSP 签名 PR #150、切换账号时隔离聊天状态 PR #151、Windows 登录后修复账号目录权限 PR #153，以及版本和发布说明 PR #155。Makepad 与 rc.11 相同，为 `8f788cbb788f287f4e684de913e83fbbb70df701`。
- [release.yml run 44384185477](https://github.com/hagency-org/hagency-desktop/actions/runs/44384185477) 使用 publish=true、macos-arm64-windows，第一次尝试中回归门禁、两平台安装包检查和发布全部通过。
- 只有两个安装包：`hagency-desktop-macos-arm64.dmg`（170,010,421 字节）和 `hagency-desktop-windows-amd64.exe`（122,292,158 字节）。公开附件已重新下载，`SHA256SUMS.txt` 和两个平台各自的校验文件都校验通过。
- 在构建机以外的一台 Mac 上，用空 profile 和隐藏窗口，通过 `tools/release-desktop-smoke.py` 运行了公开的 macOS 安装包：第二次运行通过，随包内核为 `octos 2.0.3-rc.13 (fe08d8e)`，没有资源错误。第一次运行在 37 秒时失败：那台 Mac 负载很高，刚拷贝出的内核首次启动超过了检查脚本对 `octos --version` 的 20 秒限制，与安装包无关。Windows 安装包没有在实机上运行。
- 合入五个 PR 之前，在它们与 main 合并后的代码上做了 JIT 类型检查，并在一台 Mac 上用隐藏窗口向加密房间里的本地 Agent 发了一条消息并收到回复。没有运行的：#153 的 Windows 路径、#150 的 TSP 构建、#151 的真实账号切换。
- 这次发布没有部署服务端。

## 2026-11-18 rc.11 发布

用户要求把两个登录修复（PR #139、#141）一并放进 rc.11；由 Claude 执行。

- [v0.5.0-rc.11](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.11) 于 2026-11-18 06:38:44 UTC 公开为 prerelease。标签和两个安装包都由 Desktop `bc42284ea2199d24f096452012f5b19321b94914` 构建：rc.10 之后合入了房间角色 PR #142、Windows 凭据目录检查和登录错误提示 PR #141、已保存密钥缺失时完成登录 PR #139、自有 macOS CI 机器 PR #143，以及版本和发布说明 PR #144。Makepad 与 rc.10 相同，为 `8f788cbb788f287f4e684de913e83fbbb70df701`。
- [release.yml run 44238330452](https://github.com/hagency-org/hagency-desktop/actions/runs/44238330452) 使用 publish=true、macos-arm64-windows，第一次尝试中回归门禁、两平台安装包检查和发布全部通过。
- 只有两个安装包：`hagency-desktop-macos-arm64.dmg`（169,762,543 字节）和 `hagency-desktop-windows-amd64.exe`（122,121,464 字节）。公开附件已重新下载，`SHA256SUMS.txt` 和两个平台各自的校验文件都校验通过。
- 在构建机以外的一台 Mac 上，用空 profile 和隐藏窗口，通过 `tools/release-desktop-smoke.py` 运行了公开的 macOS 安装包：检查通过。随包内核为 `octos 2.0.3-rc.13 (fe08d8e)`，没有资源错误。Windows 安装包没有在实机上运行。
- 合入 #139、#141 之前，在两者与 main 合并后的代码上做了 JIT 类型检查和登录、凭据相关测试；并在一台 Mac 上不经浏览器完成了一次真实登录：新账号注册并登录，重启后恢复，移走已保存的密钥后重启提示 `credential_missing`，再次登录成功并保留了原来的会话文件和聊天数据库。
- 这次发布没有部署服务端。

## 2026-11-18 rc.10 发布

用户要求现在出版本；由 Claude 执行。

- [v0.5.0-rc.10](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.10) 于 2026-11-18 05:19:39 UTC 公开为 prerelease。标签和两个安装包都由 Desktop `3693d969342db6b8faf0f17ce6b38e6a9e0791f6` 构建：版本和发布说明 PR #140 合入 main 后的提交，包含本地 Agent 的定时器和自发消息（PR #132、#135、#137）、把新建的本地 Agent 加入空间房间（PR #136）、睡眠或断网后本地 Agent 服务自动恢复（PR #130）、Octos 资源池（PR #122、#126）。Makepad 与 rc.9 相同，为 `8f788cbb788f287f4e684de913e83fbbb70df701`。
- [release.yml run 44224852825](https://github.com/hagency-org/hagency-desktop/actions/runs/44224852825) 使用 publish=true、macos-arm64-windows，第一次尝试中回归门禁、两平台安装包检查和发布全部通过。两个 macOS 任务等了三十多分钟才分到 runner：组织的 macOS 名额被其他 PR 的检查占用。
- 只有两个安装包：`hagency-desktop-macos-arm64.dmg`（169,616,622 字节）和 `hagency-desktop-windows-amd64.exe`（122,074,852 字节）。公开附件已重新下载，`SHA256SUMS.txt` 和两个平台各自的校验文件都校验通过；`SHA256SUMS-windows-amd64.txt` 仍是 CRLF 行尾。
- 在构建机以外的一台 Mac 上，用空 profile 和隐藏窗口，通过 `tools/release-desktop-smoke.py` 运行了公开的 macOS 安装包：检查通过。随包内核为 `octos 2.0.3-rc.13 (fe08d8e)`，包目录为 `../Resources`，包内标志已绘制，没有资源错误。
- 房间角色修复（PR #142）不在 rc.10 中，在 rc.11 中。
- 这次发布没有部署服务端。

## 2026-11-15 rc.9 发布

用户要求推送明文消息修复并发布新的 RC；由 Claude 执行。

- [v0.5.0-rc.9](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.9) 于 2026-11-15 10:21:19 UTC 公开为 prerelease。标签和两个安装包都由 Desktop `d6a85c05e55021a746d8c2842c85537ede698f28` 构建：明文消息修复 PR #114 与版本和发布说明的 PR #115 合入 main 后的提交。Makepad 与 rc.8 相同，为 `8f788cbb788f287f4e684de913e83fbbb70df701`。
- [release.yml run 43691540832](https://github.com/hagency-org/hagency-desktop/actions/runs/43691540832) 使用 publish=true、macos-arm64-windows，第一次尝试中回归门禁、两平台安装包检查和发布全部通过。
- 只有两个安装包：`hagency-desktop-macos-arm64.dmg`（169,031,496 字节）和 `hagency-desktop-windows-amd64.exe`（121,925,030 字节）。公开附件已重新下载，`SHA256SUMS.txt` 和两个平台各自的校验文件都校验通过；`SHA256SUMS-windows-amd64.txt` 仍是 CRLF 行尾。
- 在构建机以外的一台 Mac 上，用空 profile 和隐藏窗口，通过 `tools/release-desktop-smoke.py` 运行了公开的 macOS 安装包：检查通过。随包内核为 `octos 2.0.3-rc.13 (fe08d8e)`，包目录为 `../Resources`，包内标志已绘制，没有资源错误。
- 为了把 macOS runner 让给发布，取消了 #114、#115 和 main 上的 CI，发布后重跑了 main 的 Desktop CI（run 43691524389）和 Build CI（run 43691524382），两个都通过。
- 这次发布没有部署服务端。

## 2026-11-14 rc.8 发布

用户要求合并全部 PR 后发布；由 Claude 执行。

- [v0.5.0-rc.8](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.8) 于 2026-11-14 15:18:44 UTC 公开为 prerelease。标签和两个安装包都由 Desktop `217e97f7071ec00081a66e8586a4372e6cb89790` 构建：版本与发布说明的 PR #103 合入 main 后，又合入了发布修复 #108。Makepad 为 `8f788cbb788f287f4e684de913e83fbbb70df701`（`.jit/sources.lock`）。
- 第一次 dispatch（run 43563898752）没有发布任何内容。Windows 安装包在 "Package installers" 步骤失败，报 `The system cannot find the file specified`。原因：cargo-packager 0.10.1 会把包里的每个 `[[bin]]` 目标都打进安装包，不看 `required-features`，而一个仅供测试的 `[[bin]] hagency_visual_jit` 已经合入。PR #108 删除了这个目标，并让 `tools/release-assets.py plan` 遇到第二个二进制目标时报错。
- [release.yml run 43573841696](https://github.com/hagency-org/hagency-desktop/actions/runs/43573841696) 使用 publish=true、macos-arm64-windows。第一次尝试中两个平台的安装包检查都通过了，但回归门禁在 `attachment-downloads` 的一个测试上失败：测试服务器没有关闭空闲连接，与产品代码无关，后来由 PR #109 修复。只重跑失败的任务（第二次尝试）后，门禁和发布都通过。
- 只有两个安装包：`hagency-desktop-macos-arm64.dmg`（168,957,704 字节）和 `hagency-desktop-windows-amd64.exe`（121,760,487 字节）。公开附件已重新下载，`SHA256SUMS.txt` 校验通过。`SHA256SUMS-windows-amd64.txt` 是 CRLF 行尾，在 macOS 上不能直接交给 `shasum -c`；这只是格式问题，没有修。
- 在构建机以外的一台 Mac 上，用空 profile 和隐藏窗口，通过 `tools/release-desktop-smoke.py` 运行了公开的 macOS 安装包：检查通过。随包内核为 `octos 2.0.3-rc.13 (fe08d8e)`，包目录为 `../Resources`，包内标志已绘制，没有资源错误。
- 这次发布没有部署服务端。

## 2026-11-12 rc.7 发布

修复 rc.6 的 macOS 安装包；由 Claude 执行。

- [v0.5.0-rc.7](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.7) 于 2026-11-12 18:55:06 UTC 公开为 prerelease。标签和两个安装包源码统一为 Desktop `8df3b2d83f8747e6959e5a3f3092fc8f03183720`（PR #85 合入 main）。Makepad 与 rc.6 相同，为 `200eb92bdf511281ebe37bbecd732f296379af4e`。
- 修复：`build.rs` 在 macOS 的 app／dmg 打包时把包目录设为 `../Resources`，包内图片和字体从 `Contents/Resources` 读取；`src/app.rs` 在启动日志中写明资源来自包内还是源码目录；安装包冒烟检查要求包内标志图片已绘制。
- [release.yml run 43252198296](https://github.com/hagency-org/hagency-desktop/actions/runs/43252198296) 使用 publish=true、macos-arm64-windows，回归门禁、两平台安装包检查和发布全部通过。
- 恰好两个安装包：`hagency-desktop-macos-arm64.dmg`（167,611,780 字节）和 `hagency-desktop-windows-amd64.exe`（121,263,488 字节）。公开附件已重新下载，`SHA256SUMS.txt` 校验通过。
- 公开的 macOS 安装包在构建机以外的 Mac 上用空 profile、隐藏窗口经 `tools/release-desktop-smoke.py` 运行：通过，包目录为 `../Resources`，包内标志已绘制，没有资源错误，rc.6 的缺陷已消除。
- rc.6 页面没有加说明，也没有撤下其 macOS 安装包（由用户决定）。未部署服务端。

## 2026-11-12 rc.6 发布（macOS 安装包有缺陷，由 rc.7 取代）

用户要求提交 JIT 引擎线的改动、让其他人可以运行 JIT 版本，并发布新的 RC；由 Claude 执行。

- [v0.5.0-rc.6](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.6) 于 2026-11-12 15:22:00 UTC 公开为 prerelease。标签和两个安装包源码统一为 Desktop `36187b5fab526b7fa6dfa54d1692f05aa4eb798c`（PR #81 合入 main 后加 #82、#83、#84 三个发布修复）。stable latest 仍为 v1.1.2。
- Client 已在树内（`crates/hagency-client`），build report 的 clientCommit 与 sourceCommit 相同。Makepad 首次使用 JIT 引擎线：`200eb92bdf511281ebe37bbecd732f296379af4e`（OctoSense fork 分支 `hagency-desktop-0.5.0-rc.6`，记录在 `.jit/sources.lock`）。
- **发布后发现的缺陷：macOS 安装包找不到包内的图片和字体。** 在构建机以外的 Mac 上，登录页标志、图标等图片不显示，文字改用系统字体。原因：`robius-packaging-commands` 0.4.1 为 macOS 传 `MAKEPAD_PACKAGE_DIR=.`，并靠旧引擎的 `MAKEPAD=apple_bundle` 开关定位 `Contents/Resources`；JIT 线引擎不读这个开关，只按可执行文件旁的包目录查找。Windows 安装包传的同样是 `.`，资源就在可执行文件旁，不受影响。CI 冒烟检查只看登录页文字和非空画面，而字体在构建机上能从源码路径读到，所以没有发现；把公开的 DMG 下载到另一台 Mac 上运行才发现。修复：`build.rs` 在 macOS 的 app／dmg 打包时把包目录设为 `../Resources`；冒烟检查改为要求包内标志图片已绘制、启动日志声明包目录、且没有"字体未随包"错误。
- [release.yml run 43224070801](https://github.com/hagency-org/hagency-desktop/actions/runs/43224070801) 使用 publish=true、macos-arm64-windows，是第四次 dispatch。前三次没有发布任何内容：(1) run 43189892280：发布流程没有设置 `HAGENCY_NATIVE_CREDENTIAL_IO=deny`，另有两个只在 Windows 编译的断言已过期（PR #82）；(2) run 43195636756："Verify Windows graphics startup and recreation" 运行的引擎测试在 JIT 线不存在，该步骤被删除，**设备重建不再在本流程中检查**（PR #83）；(3) run 43198816202：安装包冒烟脚本仍用旧引擎的 HTTP 远程端口，一个门禁测试依赖机器上装有 Codex CLI，build report 的 makepadCommit 取自旧的 Cargo.toml 固定值（PR #84）。JIT 线在 GitHub 机器上的安装包任务 macOS 约 92 分钟、Windows 约 85 分钟，加上排队和门禁，每次尝试约 2 小时 15 分。
- 210 项预算／聊天回归全部通过。macOS Desktop 单测 995 通过、2 忽略；Windows 973 通过、2 忽略；两平台各 10 个 app-contract 测试通过。
- 恰好两个安装包：`hagency-desktop-macos-arm64.dmg`（167,713,717 字节）和 `hagency-desktop-windows-amd64.exe`（121,389,765 字节）。两个 build report、三个 SHA256SUMS 及两个安装包共 7 个公开附件已重新下载，大小和 SHA-256 均与 GitHub 元数据、平台报告及校验文件匹配；报告中的 Desktop、Client、Makepad 提交一致。匿名访问 Release 和安装包下载地址均为 HTTP 200。
- 两个安装包在 CI 上用隔离 profile 启动，连续三帧登录页非空；随包运行时为 `octos 2.0.3-rc.13 (fe08d8e 2026-11-12)`。Windows 为可见窗口，渲染截图与原生桌面截图已查看，标志正常；macOS 为隐藏窗口，截图中没有标志（即上述缺陷，当时没有被检查拦下）。没有人工 Windows 11 验收。
- Mac 仍为 ad-hoc 签名、未 notarize；Windows 未签名。升级说明见发布说明：rc.5 及更早版本的 Hosting 设置在首次启动时被丢弃；已保存的登录单向迁入系统凭据库。
- 证据：上述 run 的 artifacts（`release-regressions`、`installer-checks-*`、`candidate-installers-*`，保留 14 天）。未部署服务端，未修改服务器配置。

## 2026-11-07 rc.4 发布

用户要求发布包含最新 Agent 问题卡修复的新 RC；由 Codex 执行。

- [v0.5.0-rc.4](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.4) 于 2026-11-07 17:35:00 UTC 公开为 prerelease。标签和两个安装包源码统一为 Desktop `5db1ece7f73b1847d0848736f6777c20efca76e9`，包含 rc.3 发布分支及已复核的问题卡提交 `b0243afdd5a07bee9dab14267836ac9d1bcffcec`。stable latest 仍为 v1.1.2。
- Client 固定为 `6cd99ceb84ae407b2315dcd0f5e900022eebac06`；Makepad 保持 `a0ad1839c61cecbd7e83251cfb7d50d7a725ee48`。本次加入消息下方的问题卡，拒绝通过问题通道批准 MCP 工具或技能安装、拒绝其他 Agent 的回答；同一轮问题共用十分钟等待预算，包含加密设备打开及重试；重启或取消后恢复卡片终态，桌面端有界重试暂时无法解密的卡片，并隔离旧登录会话的回调。还包含长 Codex 运行及已完成回答的保留修复。
- [release.yml run 42387632599](https://github.com/hagency-org/hagency-desktop/actions/runs/42387632599) 使用 publish=true、macos-arm64-windows，完整执行成功。110 项预算／聊天回归全部通过，累计 812 次 Rust 测试执行（选择器有重叠，不是独立测试总数）。六组问题卡检查实际执行 54 个测试；四组 rc.3 恢复检查实际执行 21 个测试，均通过。合并回归脚本时保留了两边全部检查，没有绕过门禁或接受零测试结果。
- macOS Desktop 单测 792 通过、2 忽略；Windows 776 通过、2 忽略。两平台各 10 个 app-contract 测试通过，Windows 软件图形设备启动／重建回归实际执行 1 个测试并通过。客户端源码在发布前另通过默认 workspace 3,216 项和 shipping features 1,240 项完整测试，保留既有 ignored／filtered 配置；独立源码复核无剩余阻塞项。
- 恰好两个安装包：`hagency-desktop-macos-arm64.dmg`（162,910,613 字节）和 `hagency-desktop-windows-amd64.exe`（117,586,176 字节）。两个 build report、三个 SHA256SUMS 及两个安装包共 7 个公开附件已重新下载，大小和 SHA-256 均与 GitHub 元数据、平台报告及校验文件匹配；报告中的 Desktop、Client、Makepad 提交一致。匿名访问 Release 和两个安装包下载地址均为 HTTP 200。
- 两个实际安装包均在隔离 profile 下启动并通过连续三帧登录页非空检查，字体／资源错误为零。随包运行时为 `octos 2.0.3-rc.13 (fe08d8e 2026-11-07)`。macOS 渲染截图、Windows 渲染及原生桌面截图共三张已查看；Windows 原生窗口可见。没有执行独立 verify_only 流程，也没有宣称完成了人工 Windows 11 全流程验收或 River 真实加密房间验收。
- Mac 仍为 ad-hoc 签名、未 notarize；Windows 未签名。macOS 资源托管集成在 Desktop 的 Agent → Hosting；没有单独 Hosting 安装包、Windows 后台托管或 Intel macOS 包。发布说明保留已知用量可能低估服务商消耗、兼容服务端要求及既有依赖审计开放项。
- 发布证据位于 `~/.codex/task-reports/hagency-rc4-20261107/release.json`，包含完整工作流、110 项回归报告、两平台测试日志、安装检查、截图及公开附件核验。未部署服务端、修改 River 配置或发送真实房间消息。

## 2026-11-06 rc.3 发布

用户要求发布新的 RC；由 Codex 执行。

- [v0.5.0-rc.3](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.3) 于 2026-11-06 18:09:33 UTC 公开为 prerelease。标签与全部二进制源码统一为 Desktop `f394023b1fda46c7cfa2684771d6c8a5073eb0f7`，基于最新修复 `91cd4b252df0b0544b2407f9b0db826290247d28`。stable latest 仍为 v1.1.2。
- Client 固定为 `686d473e2ff21da784adfe117f3b17b7efd21302`；Makepad 保持 `a0ad1839c61cecbd7e83251cfb7d50d7a725ee48`。包含按已知用量恢复额度、从未启动消息的有界重试、凭证检查重试与更精确的诊断、单一 owner 目录和集成 Hosting。发布说明明确：缺失用量按零处理可能低估服务商用量并允许超额；消息重试需要服务端支持 `shared-demand-retry-v1`，不能把凭证刷新断言为所有中断的根因。
- [release.yml run 42250311709](https://github.com/hagency-org/hagency-desktop/actions/runs/42250311709) 使用 publish=true、macos-arm64-windows，完整执行成功。104 项预算／聊天回归全部通过，累计 758 次 Rust 测试执行（有重叠，不是独立测试总数）；本次新增 4 组恢复检查，实际执行 21 个测试，全部通过。没有绕过门禁或使用零测试过滤结果。
- macOS Desktop 单测 779 通过、2 忽略；Windows 763 通过、2 忽略。两平台各 10 个 app-contract 测试通过，Windows 软件图形设备启动／重建回归实际执行 1 个测试并通过。源码指针、翻译、图标和 release-assets 检查通过。
- 恰好两个安装包：`hagency-desktop-macos-arm64.dmg`（162,533,214 字节）和 `hagency-desktop-windows-amd64.exe`（117,295,611 字节）。macOS 资源托管已并入 Desktop 的 Agent → Hosting；没有单独 Hosting 安装包，也没有 Windows 后台资源托管或 Intel macOS 包。
- 两个实际安装包均用隔离 profile 启动，连续三帧登录页非空，字体／资源错误为零；随包运行时为 `octos 2.0.3-rc.13 (fe08d8e 2026-11-06)`。macOS 渲染截图、Windows 渲染及原生桌面截图共三张已人工查看，Windows 原生窗口可见。没有运行独立 verify_only 流程，也没有宣称做过人工 Windows 11 全流程验收。
- 两个 build report、三个 SHA256SUMS 加两个安装包，共 7 个公开附件已重新下载，大小和 SHA-256 均与 GitHub 元数据、平台报告及校验文件匹配；报告中的 Desktop、Client、Makepad 提交一致。匿名访问 Release 和两个安装包下载地址均为 HTTP 200。Mac 仍为 ad-hoc 签名、未 notarize；Windows 未签名。
- 发布证据在 `hagency-desktop-rc3-20261106/.run/rc3-release/release.json`，包含完整工作流、回归报告、安装检查、截图和公开附件。未部署服务端或修改 River 可信代理配置，未进行真实账号故障注入；这些不能由安装包回归结果代替。既有依赖审计开放项保留在发布说明中。

## 2026-11-05 rc.2 发布

用户要求重新发布，包含最新的 Windows 工作和聊天历史的本地修复，并要求快速完成；由 Claude 执行。

- [v0.5.0-rc.2](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.2) 于 2026-11-05 06:16:32 UTC 公开为 prerelease，没有替换 stable latest。标签和全部二进制源码统一为 Desktop `b00459166744dab2ec8433da8176b1521f064e21`（`work/hagency-desktop-local-20261028` 顶端）。
- Client 固定为 `19539818e927a2a2328eebba91d8736fab22c9c6`：Codex 的 agent-startup-history 分支（加密房间分页读取历史、原始／继任／预算三种执行驱动接入、消息和密钥事件唤醒加五秒兜底），含 Claude 的历史硬化修复 94850cad。Codex 通过 PR #4（763b5a7e）合入的内容与 Claude 的直接合并逐字节一致。
- Makepad 固定为 `a0ad1839c61cecbd7e83251cfb7d50d7a725ee48`（OctoSense fork 分支 `fix/hagency-windows-platform-tests`）：即 Codex 的 Windows 系统浏览器分支 `codex/windows-system-browser-20261104` 顶端 7c6f6cc8 加一行测试修复——`platform/src/retained_instances.rs` 的 `sum()` 在 Windows 上因 WebView2 把 serde_json 带入依赖图而类型歧义，makepad-platform 的 lib 测试无法编译，release.yml 的 Windows 图形启动检查因此失败。Codex 若继续该 fork 分支，请基于 a0ad1839。
- 桌面侧还包含待批 Agent 权限请求的应用内提醒、房间角标、Agent 标签提示和原位置卡片（859adaca），以及 Windows 系统浏览器、分隔条命中范围和光标可见性（66515d20）。`windows-single-exe-*` 分支是测试启动器，未纳入。
- 发布由 release.yml 第四次 dispatch（run 41966327510，publish=true，macos-arm64-windows，hosting=true）自动打标签并发布。前三次失败：(1) 待批提醒三条文案未进翻译目录；(2) 门禁自 1827126e 起改在 macOS 15 机器运行，`apple-chat-fonts`／`apple-system-fonts` 失败——探针（临时 probe 分支，run 41964220918，已删除）证实 macOS 15.8.2 镜像没有 PingFang 字体资产，CoreText 把 PingFangSC-Regular 指向 PingFangUI.ttc，其中只有 `.PingFangUIText…-Default` 变量字体和 Medium 字重；现在 `article_makepad::apple_fonts::installed()` 为假时相关断言测试显式跳过并打印原因，目录错误用例改为不依赖系统字体；(3) 上述 Makepad 编译错误。
- 恰好三个安装包：`hagency-desktop-macos-arm64.dmg`（154,623,798 字节）、`hagency-desktop-windows-amd64.exe`（116,828,093）、`hagency-hosting-macos-arm64.dmg`（91,986,406），附两个 build report 和三个 SHA256SUMS。回归门禁在 macOS 15 机器上全部通过；两平台打包任务内的安装启动冒烟、Windows 图形启动检查、Hosting 生命周期和语言切换检查全部通过。rc.1 额外做过的独立 verify_only 验证和人工截图检查本次没有做；Mac 仍为 ad-hoc 签名，Windows 未签名。
- 本地证据在 `hagency-desktop-context-20261104/.rc2-*.log`（按 CI 原命令复现：桌面 766 单测、契约 10、字体 7、makepad-platform 3）。Codex 历史修复的 River 真机复测仍未做，仍待用户授权。macOS 打包任务在 GitHub 2 核机器上耗时约 2 小时；用户问能否自建，River 主机（M2 Pro 10 核，macOS 14.8.5，未装 Rust）可注册为 self-hosted runner 承担 macOS 任务，Linux 机器做不了 Windows 和 macOS 包。

## 2026-11-04 RC 发布

用户明确要求更新 Makepad 并发布 macOS / Windows RC，另提供带中英文切换的 resource daemon 控制应用，并排除 Intel Mac。本次发布完成，以下状态取代 10 月 16 日的暂停发布决定；原记录保留在下方。

- [v0.5.0-rc.1](https://github.com/hagency-org/hagency-desktop/releases/tag/v0.5.0-rc.1) 于 2026-11-04 16:56:52 UTC 公开为 prerelease，没有替换 stable latest。Release ID `433792626`。标签和全部二进制源码统一为 Desktop `1827126ede1bec7b83547b871b735625b6c83890`。
- Client 固定为 `a06e4cc204989bb36e666d59f6510ae7a0489a77`；Makepad 固定为 `ebec2e5c866eae3c791e2f24d59db543cad97a05`，包含 #132 的 Windows 焦点／次级窗口重绘修复，以及软件图形设备回退、静止窗口接收异步 shader 编译结果的修复。Makepad 源码在 OctoSense fork 的 `fix/hagency-windows-software-device` 分支公开。
- 恰好三个安装包：`hagency-desktop-macos-arm64.dmg`、`hagency-desktop-windows-amd64.exe`、`hagency-hosting-macos-arm64.dmg`。Hosting 包含 controller 和 daemon，支持跟随系统／English／简体中文，即时切换并跨重启保存。没有 Intel macOS 包，也不宣称支持 Windows 原生资源托管。
- Mac 安装包在隔离 Apple Silicon worktree、macOS 26.6.2 上构建，使用 pinned cargo-packager 0.10.1。Mac CI 未分配到运行器即因容量不足被取消，因此没有宣称整轮跨平台 CI 成功。Windows [job 126660061999](https://github.com/hagency-org/hagency-desktop/actions/runs/41854657931/job/126660061999) 的应用测试、图形设备启动／重建、打包、安装和真实渲染检查全部 success；所属 run 41854657931 总状态为 failure，不能把它写成整轮 green。
- Mac 760 个 Desktop 单测、10 个 app-contract 单测通过；完整回归 95/95 组、699 次 Rust 测试执行通过（与单测存在重叠，不是合并后的独立测试总数）。翻译检查 2804 条、1596 调用点、172 权限，缺失 0。隔离测试首次因脚本 VM 的 target 路径不匹配而停止，保留失败证据后修正构建路径并从首个未通过检查续跑，没有改动产品源码或跳过检查。
- 实际两个 Desktop 安装包均以干净 profile 启动，连续三帧登录页非空且资源加载正常；Windows 原生桌面截图与渲染截图一致。实际 Hosting DMG 中的 controller／daemon 哈希与构建记录一致，7 项生命周期检查、8 项语言切换／重启保存检查通过。六张实际截图已人工检查。
- Release 附有两个 build report、三个 verification report，以及每平台和总 SHA256SUMS；11 个公开附件的哈希和大小逐一匹配。匿名访问 Release 成功，标签确认为上述源码提交，三个安装包下载地址均返回 HTTP 200。
- Mac 为 ad-hoc 签名、未 notarize；Windows 未签名。发布说明已明确这些限制，保留既有依赖审计的开放事项，没有声称所有上游依赖告警已修复。

## 2026-10-16 历史记录

更新：2026-10-16。

- 用户确认发布目标为 hagency-desktop，要求版本 0.5.0，取代继承的 1.1.2；Cloud 不修改版本。
- hagency 依赖改为 https://github.com/chrislearn/hagency-client.git 的 main 分支；Cargo.lock 锁定 39e990c8d483d6bd29b4135742f486e7b83ad137，14 个引擎 crate 均来自 Git。删除 sibling checkout CI action 和全部调用，构建仍用 --locked。
- Windows cargo check --locked --lib 通过；i18n 缺失 0、主题检查通过。五平台测试及七安装包由 GitHub release.yml 的 publish=true 手动运行验收，全部成功后自动打标签和发布，不能把 dispatch 当成发布成功。
- 发布说明在 packaging/release-notes.md，说明功能、版本回退及 Agent 加密 Room、签名和移动端限制。发布显式 --latest，保证低于 1.1.2 的 0.5.0 仍成为下载入口。
- Cloud 新增 docs-tenant/{en,zh}/agents.md 和 downloads.md，包含七个官方 latest/download 安装包链接；其发布进度在 Cloud dev-vibe/memory/desktop-downloads-and-agents.md 记录。

首轮安装包运行38832134391的macOS ARM64编译和459单测通过，但打包app失败：cargo-packager0.10.1 No matching IconType。此前品牌改动将普通1024 PNG加入icons但未列现成ICNS；上游create_icns_file对1024@1x转换报错，显式ICNS会优先原样复制。现把resources/icon.icns列入Cargo packager icons（Linux仅取PNG、Windows优先ICO，不受影响），新增tools/check-release-icons.py及CI／发布前置校验。Windows locked库编译与真实图标输入检查通过。必须用修复提交重新完整构建后发布，不能重用旧提交混合附件。

最新状态（覆盖此前发布计划）：因发布审计未达标，38832134391、38837925791均取消，没有v0.5.0标签或公开Release。Desktop主lock的Rustls0.23.46、webpki0.103.15、AWS-LC1.18.2/sys0.46.0、Quinn0.11.18兼容安全补丁将漏洞匹配24降到5，仍21其他警告；Windows locked check与456单测通过（2忽略）。随包Octos fe08独立Cargo.lock仍36漏洞匹配+14其他警告，Desktop补丁不会传递过去。实际PDF抽取及RSS属性解析存在需修补路径；不能把所有完整锁文件记录都当成已证实可利用。报告与机器证据见../release-0.5.0/assessment.zh.md。已询问用户是否扩大范围修复上游后再发，未获答复前不再启动正式发布；Cloud上线同样待其独立确认。

最终验证：7ad1b8dca67222bd8312952799b3dc397e12062a的主CI38842267586与五平台构建38842267688均success（远端Mac461pass/2ignore、本地Win456pass/2ignore），翻译缺失0、主题与图标前置检查通过。正式发布仍暂缓；未获新的上游升级范围答复，不自动开启第三轮发布或扩改上游。独立Octos锁已与fe08的Git对象逐字节比对，SHA256 cfed26d0a98f515574607f6015a9547caccace48df43f510c2df38063fe45e64。
