# Plan 008: 修正缓存有效性判定，空文件与 .html 不再无条件有效

> **执行者须知**：逐步执行，每步跑验证命令确认预期后再继续。触发「STOP 条件」立即
> 停止汇报。完成后更新 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 状态行。
>
> **漂移检查（先跑）**：`git diff --stat 82d2203..HEAD -- crates/emukc_cache/src/kache.rs`

## 状态

- **优先级**: P1
- **工作量**: S
- **风险**: MED
- **依赖**: 001
- **类别**: bug
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

`is_valid` 里有两条「一律算有效」的捷径，它们让**错误内容被永久缓存**：

1. **零字节文件被显式判定为有效**。CDN 返回 200 但 body 为空时，落盘一个 0 字节
   文件、写入 version 行、此后一直命中本地缓存。对清单中约 39,400 个**不带版本号**
   的路径，`find_in_local` 只看 `is_valid`，没有版本兜底，这个空文件就是永久的。
2. **`.html` 扩展名跳过全部校验**。该函数下面有一段专门用来识别「HTML 错误页」的
   嗅探（检查 `<!DOCTYPE html>` / `<html`），但 `.html` 文件在到达那里之前就
   `return true` 了。而走 gadget CDN 的恰恰是 `gadget_html5/` 和 `world.html`
   这类路径——需要这个嗅探的正是它们。

这不是崩溃场景，是稳定可复现的：任何一次「200 + 错误页」或「200 + 空 body」都会
被当成成功缓存下来。

**当前实测**：`find z/cache -type f -size 0` 返回 0 个（4,207 个已缓存文件中
没有空文件），所以这是尚未触发的洞，不是正在流血的伤口。修它的理由是成本极低
而后果不可逆（缓存一旦污染，只能人工删文件）。

## 当前状态

`crates/emukc_cache/src/kache.rs:596-635`：

```rust
    async fn is_valid(path: &std::path::Path) -> bool {
        if !path.exists() || !path.is_file() {
            trace!("File does not exist or is not a file: {:?}", path);
            return false;
        }

        // HTML files are always valid
        if path.extension().is_some_and(|ext| ext == "html") {
            trace!("File is a HTML file: {:?}", path);
            return true;
        }

        trace!("File is not a HTML file: {:?}", path);

        let Ok(mut file) = tokio::fs::File::open(path).await else {
            trace!("Failed to open file: {:?}", path);
            return false;
        };

        let Ok(metadata) = file.metadata().await else {
            return false;
        };

        // Empty files are valid
        if metadata.len() == 0 {
            return true;
        }

        // Read first 512 bytes to detect HTML error pages
        let read_size = 512.min(metadata.len() as usize);
        let mut buffer = vec![0u8; read_size];
        if file.read_exact(&mut buffer).await.is_err() {
            trace!("Failed to read file: {:?}", path);
            return false;
        }

        // Check if content looks like HTML error page
        let content = String::from_utf8_lossy(&buffer);
        !content.contains("<!DOCTYPE html>") && !content.contains("<html")
    }
```

`crates/emukc_cache/src/kache.rs:519-525`，`is_valid` 失败时**不删除**坏文件：

```rust
        if !Self::is_valid(local_path).await {
            error!("invalid file: {:?}", local_path);
            return Err(Error::InvalidFile(local_path.display().to_string()));
        }
```

`crates/emukc_cache/src/kache.rs:320-328` 选择 gadget CDN 的条件覆盖
`gadget_html5` / `html` / `world.html` 路径——也就是会跳过校验的那一批。

**清单构成实测**（`z/cache/cache_resources.nedb`，94,558 行）：约 55,100 行带
`version` 字段，约 39,400 行不带。不带版本的那批只靠 `is_valid` 把关。

**仓库约定**：Rust edition 2024，软 tab 4 空格。

## 需要用到的命令

| 用途 | 命令 | 成功时的表现 |
|------|------|--------------|
| 缓存测试 | `cargo test -p emukc_cache` | 全部通过 |
| 全量测试 | `cargo test` | 全部通过 |
| 格式 | `cargo fmt --all --check` | exit 0 |
| Clippy | `cargo clippy --workspace -- -W warnings` | exit 0 无 warning |

## 范围

**范围内**：

- `crates/emukc_cache/src/kache.rs`（只改 `is_valid` 与 `fetch_from_url` 中
  `is_valid` 失败后的清理）
- `crates/emukc_cache/tests/validity.rs`（翻转计划 001 留下的断言）

**范围外**：

- `crates/emukc_network/src/download.rs` 的写入路径。审计初稿说它会留下 0 字节
  文件，**经核验不准确**：`download.rs:346` 先 `response.bytes().await` 把整个
  body 读进内存，再 open/truncate/copy，所以网络中断不会留下半截文件
  （那行注释说的就是这件事）。真正的洞在 `is_valid`，本计划就修这里。
  给下载路径加「临时文件 + rename」是合理的加固，但属于独立改动，不要夹带。
- 错误分型（`FileNotFound` vs `FailedOnAllCdn`）——计划 007。
- redb 的 schema——不动。

## Git 工作流

- 分支：`fix/cache-validity-checks`
- 提交信息：`fix(cache): stop treating empty and .html responses as valid`
- 不 push，不开 PR，除非操作者要求。

## 步骤

### 步骤 1：先确认没有合法的零字节资源

改判之前必须确认「空文件一律无效」不会误杀真实资源。在现有缓存上查：

```
find z/cache -type f -size 0 | head -20
find z/cache -type f -size 0 | wc -l
```

**预期**：0 个。如果**不是** 0，逐个看它们是什么路径——如果存在合法的零字节游戏
资源，按 STOP 条件汇报，不要直接改判。

**验证**：命令输出 0，或者把非零结果汇报出来

### 步骤 2：去掉「空文件有效」

把 `metadata.len() == 0 { return true; }` 改成返回 `false`，并把注释改成说明理由
（零长度响应是 CDN 异常，不是合法资源）。

**验证**：`cargo build -p emukc_cache` → exit 0

### 步骤 3：让 .html 也接受校验

删掉 `.html` 的提前 `return true`，让 HTML 文件也走到下面的内容嗅探。

**但直接这么改会误杀**：合法的 `world.html`、`gadget_html5/` 页面**本身就是
HTML**，会被 `<!DOCTYPE html>` 嗅探判为无效。所以嗅探逻辑必须按扩展名分流：

- **非 HTML 扩展名**（.png/.json/.mp3 等）：内容看起来像 HTML → 无效
  （这就是现有的错误页嗅探，保持）。
- **HTML 扩展名**：不能用「像不像 HTML」判断。改用一个最小的健全性检查，
  例如长度下限（一个 CDN 错误页通常远小于真实页面），或检查是否含已知的错误页
  标记。**选择哪种要有依据**：先看 `z/cache` 里现有的几个 html 文件有多大：

  ```
  find z/cache -name '*.html' -exec ls -l {} \;
  ```

  据此定一个保守的下限，并在代码注释里写明这个数字是怎么来的。

如果看完实际文件发现没有可靠的判据，**采用保守方案**：HTML 仍然只做
「非空」检查（即步骤 2 的改动对它同样生效），不做内容嗅探，并在注释里说明
为什么。这仍然比现状好——现状是连空文件都算有效。

**验证**：`cargo build -p emukc_cache` → exit 0

### 步骤 4：无效文件落盘后要删掉

`fetch_from_url` 中 `is_valid` 返回 false 时，**先删除该文件**再返回
`Err(Error::InvalidFile(..))`。否则那份坏内容留在磁盘上，下次
`find_in_local` 对不带版本的路径可能又把它当成命中。

删除失败只记 `warn!`，不要覆盖原本要返回的错误。

**验证**：`cargo build -p emukc_cache` → exit 0

### 步骤 5：翻转计划 001 的断言

计划 001 在 `crates/emukc_cache/tests/validity.rs` 和 `remote_fetch.rs` 里留了
带「计划 008 会翻转此断言」注释的用例。逐条翻转并删掉注释：

- 200 空 body → 现在应当**失败**，且磁盘上**不留文件**
- 零长度本地文件 → 判为无效
- `.html` 且内容是错误页 → 按步骤 3 选定的方案断言
- 非 `.html` 但内容像 HTML → 仍然无效（不回归）
- 正常非空文件 → 仍然有效（不回归）

新增一个：`is_valid` 失败后磁盘上没有残留文件（验证步骤 4）。

**验证**：`cargo test -p emukc_cache` → 全部通过

### 步骤 6：在真实缓存上做一次回归确认

用现有的、已经填充了几千个文件的缓存跑一小批清单，确认新判定不会把**已经缓存好
的正常文件**判成无效导致重新下载：

```
head -200 z/cache/cache_resources.nedb > /tmp/validity-check-200.nedb
cargo run --release -- cache populate --src /tmp/validity-check-200.nedb --concurrent 8
```

**预期**：绝大多数条目命中本地缓存、不重新下载，失败数为 0。如果出现大量重新
下载或 `invalid file` 错误，说明判定过严，按 STOP 条件汇报。

**验证**：无 `invalid file` 错误，失败数 0

### 步骤 7：全量门禁

**验证**：
- `cargo test` → 全部通过
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace -- -W warnings` → exit 0
- `git status` → 只改范围内文件

## 测试计划

见步骤 5。不新建测试文件，在计划 001 建好的 `validity.rs` / `remote_fetch.rs`
里翻转和补充。

## 完成标准

- [ ] `is_valid` 中不再有 `metadata.len() == 0 { return true; }`
- [ ] `is_valid` 中不再有基于 `.html` 扩展名的提前 `return true`
- [ ] `is_valid` 失败时坏文件被删除
- [ ] 计划 001 留下的「计划 008 会翻转」注释已全部消除
- [ ] 步骤 6 的真实缓存回归中没有出现 `invalid file`，失败数为 0
- [ ] `cargo test` exit 0
- [ ] `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings` 都 exit 0
- [ ] README.md 状态行已更新

## STOP 条件

- 计划 001 未完成。
- 步骤 1 发现缓存里**存在**合法的零字节资源——汇报路径清单，改判会误杀它们。
- 步骤 3 中看完真实 html 文件仍找不到可靠判据，且保守方案也被评审否决。
- 步骤 6 出现大量重新下载或 `invalid file`——说明判定过严，汇报具体路径和原因，
  不要靠放宽阈值反复试。

## 维护须知

- HTML 的校验是本计划最脆的一环：合法页面和错误页都是 HTML，没有内容无关的判据。
  如果以后发现更可靠的信号（例如 CDN 错误页有固定标记），应当替换掉长度阈值。
  代码注释里必须留下当前判据的来源。
- 评审重点：确认非 HTML 路径的错误页嗅探**没有被削弱**——那是这个函数原本就做对
  的事。
- 明确推迟：给 `emukc_network` 的下载写入路径加「同目录临时文件 + rename」。
  当前实现先把 body 读进内存再落盘，中断窗口极小；计划 005 已经给 bootstrap 的
  web 资产做了原子替换，那份实现可以作为将来推广到资源下载路径的参照。
