# Bootstrap / Cache 审计实施计划

由 `/improve` 于 2026-09-20 生成，审计基线 commit `82d2203`。
范围：`emukc_bootstrap`（下载、解析、make-list）、`emukc_cache`、`emukc_network`
的下载层，以及 bootstrap → decode-main → make-list → populate 的编排。

每个执行者：完整读完计划再动手，遵守其 STOP 条件，完成后更新下表自己那一行。

## 执行顺序与状态

| Plan | 标题 | 优先级 | 工作量 | 依赖 | 状态 |
|------|------|--------|--------|------|------|
| 001 | 为 emukc_cache 写入/过期/失败路径建立 mock CDN 测试基线 | P1 | M | — | DONE |
| 002 | 为第三方数据解析器建立 fixture 测试基线 | P1 | M | — | DONE |
| 003 | 恢复 HTTP 连接池复用并去掉每文件多余的 HEAD | P0 | S | 001 | DONE |
| 004 | 重写 kccp 任务解析器，消除状态机失步 | P0 | S | 002 | DONE |
| 005 | bootstrap web 资产改为先下后替，失败时硬报错 | P0 | S | — | DONE |
| 006 | 禁止空需求被判定为「任务已完成」 | P0 | M | 002 | DONE |
| 007 | 区分「资源不存在」与「瞬时网络失败」 | P1 | M | 001 | DONE |
| 008 | 修正缓存有效性判定：空文件与 .html 不再无条件有效 | P1 | S | 001 | DONE |
| 009 | populate 失败清单落盘，并把 404 从重试路径里分流出去 | P1 | S | 007（仅步骤 4） | DONE |
| 010 | 删除 Greedy / holes-report 死代码并修正文档 | P1 | S | — | DONE |
| 011 | 修复 label_type 年任务表，未命中改为硬错误 | P1 | S | 002 | DONE |
| 012 | 为 cache-list 增加客户端版本校验 | P1 | S | — | DONE |
| 013 | 设计单一权威的客户端版本记录（spike） | P2 | M | 012 | DEFERRED |

状态取值：TODO | IN PROGRESS | DONE | BLOCKED（附一行原因）| REJECTED（附一行理由）

- 004 DONE：`parse` 从「按行 + 正则 + 三状态机」换成「按条目顺序分组」。没有用计划
  优先的方案 1——`serde_json/preserve_order` 是 feature unification 下全工作区生效的，
  会改掉 27 处 `serde_json::Map` 的迭代与序列化顺序（含冻结的 battle golden），为一个
  解析器付这个代价不划算。也没有手写方案 2 的转义扫描，而是用 `serde` 的 `MapAccess`：
  `visit_map` 本来就按文档顺序交付条目，既保序又让 serde_json 去处理转义。
  分组规则：`_quest_id_N` 开新记录，`"dummy": "forNoComma"` 直接丢弃，其余归入当前记录；
  2 条目按 name/desc，1 条目按启发式（以「！」「。」结尾或超过 30 字符判为 desc）并 `warn!`，
  0 条目 `warn!` 跳过，超过 2 条取前两个并 `warn!`。缺失的一侧留 `"n/a"`，不拿描述冒充标题。
  实测（771 条全量，临时 `#[ignore]` 测试跑完已删）：771 个 id 解析出 771 条记录，
  缺 name 的 11 个 id 与计划逐字一致（256/615/616/622/627/628/630/632/633/648/652），
  缺 desc 的 2 个也一致（1124/1125），没有任何字段泄漏内部 key。启发式 13/13 正确。
  重跑 `bootstrap --overwrite` 后：`grep -c '"_quest_id_' .data/codex/quest.json` 由 8 变 0，
  日志里 `quest info not found` 由 8 条降到 4 条（剩下的正是上游确实没有的），
  `has a single entry` 的 warn 恰好 13 条。
  **一处计划措辞需要更正**：步骤 5 说修复后 name 为 `n/a` 的不再包含 257/616/623/628/
  631/633/649/653 这 8 个。实际 616、628、633 仍是 `n/a`——它们本来就属于 kccp 里缺 name
  的那 11 条，不是失步受害者。修好的是 257/623/631/649/653 这 5 个被连带毁掉的下一条。
  同时 256/615/622/627/630/632/648 由「拿描述冒充标题」改为如实的 `n/a`，所以 codex 里
  `name == "n/a"` 的条数从 12 变成 15（= 11 条 kccp 缺 name + 4 条上游完全没有）。
  这与计划「维护须知」里说的 11 条一致，是预期结果。

- 013 DEFERRED（2026-09-21）：维护者选了「先接通，再谈收敛」，spike 不写。理由是 013 的核心
  证据——drift-check 三次被绕过——的原因是它没接进任何流程，再补一份设计文档是同一个失败模式。
  已落地的替代动作见 `617cfca`：跟踪集 6 → 13（补上 7 个 cache-list 输入）、基线
  `6.3.0.0 → 6.3.5.0`、`make update` 打不阻断的漂移报告、`make drift-check` 为门禁形式、
  `make drift-accept` 为刻意刷新。四份版本记录**仍未收敛**，本计划的三个设计问题原样留着；
  重新评估的判据是接通之后它是否真的被用起来（报告有没有人看、基线有没有第四次被绕过）。

- 012 DONE：decoder 侧 `extractResourceManifest` 加了 `scriptVersion` 参数并写进产物，
  `resource_manifest.json` 经 `bun run decode -- --sync-resource-manifest` 重新生成（未手改），
  现在 9 个资产全部带这个字段。Rust 侧 `ResourceManifest` 用
  `#[serde(default)] script_version: Option<String>` 接收，旧格式仍可加载。
  校验放在 `make_list::make` 里、`build_list` **之前**：拉实时 `scriptVesion`，按策略取对应
  资产的 `scriptVersion`（Manifest 取 `resource_manifest.json`，Default/Rules 取
  `cache_rules.json`），不一致就返回错误。判据抽成纯函数 `asset_freshness`，有表驱动测试。
  没有为此重构生成流程——校验多加载一次本地 JSON，换取生成路径一行不动。
  写文件确实在 `build_list` 之后，所以中途报错不会留下半份清单。
  **计划遗漏的调用点**：`src/bin/cli/auto/mod.rs` 也调 `make_cache_list`（`cargo build --workspace`
  不编译 bin，所以只有 `--release` 才暴露）。auto 是首次运行的便利流程，传 `true` 放行——
  在那里因资产过期卡住等于让人装不上游戏。
  `pipeline.ts` 的 sync 分支原本重复调用了一次 `extractResourceManifest`，改为复用
  `result.resourceManifest`，控制流不变。
  实测：情况 A（版本一致）exit 0 生成 73,031 条；情况 B（把 `cache_rules.json` 的
  `scriptVersion` 临时改成 `0.0.0.0`）非零退出、**没有**产出清单文件，错误信息同时给出
  `0.0.0.0` 与 `6.3.5.0` 和修复命令；加 `--allow-stale-assets` 后 exit 0 并打 WARN。
  测完已 `git checkout` 还原该资产。TypeScript 侧 `bun run check` + `bun test` 61 pass。

- 011 DONE：两张「任务编号 → 月份」表换成 `label_type = 100 + release_date 的月份`，
  `By5` 作为唯一特例保留（发布 2020-09-17 却标 7 月）。`wiki_id` 的解析也跟着简化——
  现在只有周期字母有用，类别字母和编号都不再参与判断。
  **计划低估了影响面**：正文只统计了 `By*` / `Cy*`，说 7 个年任务受影响。实际年任务有
  **55** 条，还有 `Dy*`(8) / `Fy*`(11) / `Gy*`(4) 三类，而 `match category` 只有 `"B"` 和
  `"C"` 分支，这 23 条**全部**落到 `label_type = 1`。所以修复前错的是 30 条而不是 7 条。
  交叉验证按类别重做：B/C 已映射的 25 条里只有 By5 与发布月份不符，与计划结论一致；
  只有 By16 缺 `release_date`，按计划方案 (c) 返回 101 并 `warn!`。
  **`s` 周期不改，计划 002 的注记和项目记忆里「011 必须覆盖 s」的判断不成立**：
  `Cs*` 的 `frequency` 是 `seasonal`，而本仓库的 `From<Frequency> for Kc3rdQuestPeriod`
  把 `Seasonal` 映射为 `Oneshot`，`label_type = 1` 正是客户端的一次性标签页——两者一致。
  测试里把这条写成了有意为之，不再标「011 会翻转」。
  实测：临时脚本对全部 55 条年任务跑新逻辑，全部落在 101..=112，无一返回 1，
  除 By5 外无规则违例；重跑 bootstrap 后 codex 里年任务的 label_type 无一越界。
  新增的越界防护测试覆盖畸形 `release_date`（`2024`、`2024-13-01`、`2024-00-01`、空串）。

- 010 DONE：`CacheListMakeStrategy::Greedy`、`GreedyConfig`、五个死探测函数、
  `batch_check_exists` / `MAX_CHECK_SIZE`、`make_list/progress.rs`、两套 holes-report
  机制（`HOLES_COLLECTOR` 三件套与整个 `holes_report.rs`）、`--greedy` / `--concurrent`
  两个 flag 全部删除。`voice.rs` 的 `#![allow(unused)]` 也删了——它当初就是用来压住这些的。
  **计划遗漏的一处**：`examples/decoder_cachelist_compare.rs` 自己构造 `Greedy`
  （`BaselineStrategy::Greedy` + `--concurrent`），计划的范围清单没列它。一并删除，
  该 example 的另外两个 baseline 不受影响。
  **计划的一条完成标准是错的，没有照做**：「`grep -rn 'holes'` 无输出」会误删活代码——
  `ShipPathHoles` / `EVENT_SHIP_HOLES` / `event_ship_holes` 是 manifest 规则里的跳过表，
  `generate.rs` 与 `types.rs` 的测试正在用。实际删的判据是「holes **report**」：
  `holes_report` 与 `HOLES_COLLECTOR` 两个标识符，grep 它们才是空的。
  删除连带暴露出一批死代码，按计划步骤 1「一并清理」处理：`map.rs` 的 `MapInfoJson` 与
  `find_in_local_then_remote`（原是 `get_event_area_greedy` 的支撑）、
  `progress.rs` 的 `MAKE_LIST_STYLE` / `make_list_style`（只服务于已删的 `ProgressTracker`）、
  以及 `kcs::make` / `kcs/voice::make` / `kcs2/versioned::make` / `img::make` / `use_item::make`
  五处签名里不再被使用的 `mst` / `cache` 参数及其调用点。`img::make` 仍需要 `cache`
  （`get_cached_version`），只去掉了 `mst`。
  实测：`cargo build --workspace --examples` 零警告；`cache make-list --help` 不再有
  `--greedy` / `--concurrent`；删除前后各生成一次清单，73,031 条 **逐行相同**
  （签名简化之后又验证了一次，仍然 IDENTICAL）。
  `docs/solutions/conventions/rules-default-strategy.md` 按计划改写而非删除：保留
  `Default == Rules` 那节，把 Greedy 那节改成一条带日期的历史记录，说明它为何做不到
  自己声称的行为、以及为什么不要从 git 历史里捡回来。

- 008 DONE：`is_valid` 的两条捷径都去掉了。空文件改判无效，并且这个检查现在排在
  扩展名分流**之前**，所以 `.html` 也受它约束。`fetch_from_url` 在 `is_valid` 失败时
  先删文件再返回错误——不删的话，对那 ~39k 条不带版本的路径，`find_in_local` 下一轮
  会把同一份坏内容再服务一次。
  **HTML 只做非空检查**，这是计划步骤 3 的保守方案，依据是真实缓存里的
  `kcs2/hc.html` 只有 54 字节、内容就是 `<!DOCTYPE html><html><head></head><body></body></html>`——
  长度下限和 doctype 嗅探都无法把它与错误页分开。非 HTML 路径的错误页嗅探原样保留。
  计划 001 埋的两条「008 会翻转」断言：`zero_length_file_is_currently_valid` 翻转为
  `zero_length_file_is_invalid`，`fetch_200_with_empty_body_is_currently_accepted` 翻转为
  `fetch_200_with_empty_body_fails_and_leaves_no_file`；`html_extension_skips_the_content_check`
  在保守方案下结论不变，改名为 `html_error_page_still_passes_because_html_cannot_be_sniffed`
  并写明这是有意的。新增 `zero_length_html_is_invalid_too`。三条新断言都经变异验证。
  **步骤 1 的实测与计划预期不符，但不构成 STOP**：`find z/cache -type f -size 0` 返回的不是
  0 而是 1——`kcs/sound/kcwjcrloeyiyxw/158288.mp3`。curl 经代理跟随 301 后确认，上游对它
  稳定返回 `200` + `content-length: 0`，两个镜像一致。0 字节的 mp3 不是「合法的零字节游戏
  资源」，是这个 bug 的产物，所以按计划改判。
  **已知副作用**：该条目现在每轮 populate 会失败一次（文件被删、重下仍是空、再删）。
  它进的是 `*.failed.nedb` 而不是 `*.missing.nedb`，因为「200 + 空 body」不是 404，
  009 的分流判据接不住它——于是 20 个 CDN × 2 pass 一共重试 40 次，白白花掉约 20 秒。
  把「稳定的空 200」也归入 missing 类是合理的后续改动，但那要动 `classify_failure` 的判据，
  不属于 008。
  步骤 6 实测（真实 `z/cache`，随机 199 个已缓存文件 + 上述空文件）：`invalid file` 只出现在
  那一个路径上，199 个正常文件全部本地命中、无一重新下载。

- 006 DONE：`ParseError::EmptyRequirement { reason }` 接到既有的「跳过这条任务」路径
  （与 `UnknownCategory` 同粒度，不向上传播炸掉整轮 bootstrap）。14 个降级点里 13 个判为
  A 类改成返回该错误，只有 `extract_list` 在 `list: None` 时的 `Ok(vec![])` 是 B 类保留——
  它是「这条需求没有嵌套列表」，不是失败。
  **一处计划未预见的连坐**：照计划直接把错误用 `?` 传播，api_no 1019（B205）会被误杀。
  它是 `or` 下两个分支，第一个缺 `sortie` 块，第二个完全合法（4-5/5-5/6-5 boss S 胜 2 次），
  改前靠第二个分支可完成。因此 `extract_list` 按类别区分：`or` 的分支解析失败只丢该分支并
  `warn!`，`and`/`then` 仍然整条传播——前者只减少完成路径（更严格，无白送风险），后者会
  漏掉一个必须满足的条件（更宽松，正是本计划要防的）。所有分支都失败时仍返回错误，
  避免退化成 `OneOf([])`。
  实测（`bootstrap --overwrite` 两轮）：空需求任务由 `[1033]` 变为 `[]`；任务总数 649 → 648，
  只少 1033 一条；1019 保留且 `requirements` 与改前逐字一致；日志有 1 条
  `dropping unresolvable branch of an 'or' requirement`、1 条
  `skipping quest with unresolvable requirements`，reason 均为
  `sortie requirement must have a 'sortie' field`。
  **一条现存测试语义翻转**：`simple_category_succeeds` 断言的正是本计划要改成错误的 A 类
  路径（`Simple` 无 `subcategory` 降级为 `And([])`），按计划步骤 4 改为
  `simple_category_without_subcategory_is_rejected`。`and_category_with_empty_list_succeeds`
  测的是 B 类（`list: None`），不受影响，原样保留。
  `progress.rs` 未改动。`cargo test --workspace` 全绿 0 ignored，fmt clean，
  clippy 的 6 条 `result_large_err` 全在未触及的文件（`BootstrapDownloadError` / `Response`，
  都不含 `ParseError`），与本计划无关。

- 002 DONE：新增 `tests/fixtures/kccp/quests_sample.json`（7 个 id，覆盖正常三段式、
  缺 name 的 615/616 连对、缺 desc 的 1124、`dummy` 哨兵的 1169）与
  `tests/fixtures/kcwiki/` 五个源的小样本（4 舰 / 4 装备 / 60 使用道具 / 4 深海栖舰 /
  5 深海装备，共 43 KB）。kccp 的 5 个测试锁定当前错误行为：615 的 desc 是字面量
  `_quest_id_616` 且 616 缺失、1124 的 desc 是字面量 `_quest_id_103` 且 103 缺失、
  7 个 id 只产出 5 条，每条都带「计划 004 会翻转」注释。label_type 的 5 个测试覆盖
  d/w/m/q 四个周期、By/Cy 命中各 4 例、未命中的 7 个年任务 id（带「计划 011 会翻转」
  注释），外加计划未提到的第五种周期 `s`——Cs1/2/3/5/6 共 5 个真实 wiki_id，`match`
  里没有这个分支，同样落到 1。kcwiki 的三个测试改读仓库内 fixture，并删掉了四处写
  `.data/temp/*.json` 的调试残留（步骤 4 方案 1，没有用 `#[ignore]`）。fixture 刻意
  只选「没有改修配方」的装备，避免把整张改修引用图拉进样本。
  实测：`cargo test -p emukc_bootstrap` 233 passed / 0 failed / 0 ignored；
  clippy 警告集合与 `main` 基线逐字节相同。
  **未达成的完成标准**：「`.data/` 移走后 crate 测试 exit 0」。本计划把无 `.data`
  的失败从 7 个降到 4 个（修掉的正是 kcwiki 那三个），剩下 4 个全在本计划范围外——
  `make_list::tests` 两个要完整的 `.data/codex`，`map_pipeline::kcdata::tests` 两个
  遍历 `.data/temp/kc_data/_map` 的全量地图，都不是小 fixture 能替代的。

- 009 DONE：pass 1 的失败项改为三分（rollback / missing / retryable），404 不再进
  pass 2；pass 2 的失败项按同一判据再分一次（一条可能在 pass 1 超时、pass 2 才拿到
  404）。`*.failed.nedb` 与 `*.missing.nedb` 分开落盘，后缀不叠加（拿 failed 清单
  重跑仍写回同一文件）。摘要新增 `Missing (404): N`，退出判定改为
  `failed + missing > 0`——不改这一处的话，404 移出 `failed_count` 后全 404 的一轮
  会退化成 exit 0，正是本计划否决的方案。实测：2 条清单（1 个 404 + 1 个正常）
  → `Missing (404): 1`、无 Retried、exit 1、清单可被 `--src` 读回。

- 005 DONE：`--force-update` 不再先删 `main.js` / `version.json` / `kcs_const.js`
  （整段删除），flag 保留，语义改为「即使本地已有也重下」，实现是把
  `overwrite || force_update` 传给 `download_web_assets` 的 `overwrite` 参数——
  这与改前等价：改前 `--force-update` 靠「删掉再下」达到同样效果，所以
  「不带 `--force-update` 时是否重下」仍只由 `--overwrite` 决定。
  `download_web_assets` 改为下载到同目录的 `*.part` 再 `rename`，失败即删临时文件、
  原文件不动；CDN 全失败或未配置都记入失败列表，结尾返回
  `BootstrapDownloadError::WebAssetUnavailable`，Phase 4 的 `?` 因此真正生效。
  实测场景 A（`game_cdn` 指向不可达主机 + `--overwrite --force-update`）：exit 1，
  `main.js` 的 md5 与 mtime 均不变，无 `.part` 残留，"Bootstrap completed
  successfully." 不打印；场景 B（配置还原）：exit 0，三个资产 mtime 全部刷新，
  无 `.part` 残留。两个新单测经变异验证（去掉空 CDN 记失败 / 去掉结尾 return Err
  均使其失败）。**未覆盖**：`rename` 的成功路径没有单测，计划禁止引入 HTTP mock，
  只有场景 B 的手工实测作证。

- 007 DONE：`fetch_from_remote` 的 404 分支改返回 `KacheError::FileNotFound`，
  `exists_on_remote` 改为三态 `RemoteExistence{Present,Absent,Indeterminate}`。
  实际调用方是 4 处而非计划正文里的 1 处（`gauge.rs`、`map.rs` ×2、
  `make_list/mod.rs`）；后 3 处原本就用 `?` 传播，改后 `Indeterminate` 继续传播，
  行为不变。真正的缺陷只在 `gauge.rs` 的 `unwrap_or(false)`，现按计划的方案 (a)
  让整个 make-list 失败。回归测试 `variant_crawl_fails_instead_of_truncating_when_no_cdn_answers`
  经变异验证（把 `Indeterminate` 改回 `break` 即失败）。

- 003 DONE：两处配置改动在 `96f7689` 落地，001 的回归基线也已就位——
  `remote_fetch.rs` 的 404 用例通过，且 `fetch_200_writes_body_and_records_version`
  断言「一次 get 只产生一个请求」，把 `skip_header_check(true)` 钉住了（变异验证：
  改回 `false` 后该文件 3 个测试失败）。2026-09-21 补上了缺失的对照实测：随机 200 条
  清单上改前 30.70s / 23.35s，改后 18.13s / 13.72s，均值 27.03s → 15.93s（7.4 → 12.6
  files/s），失败数都是 0。**样本选择是关键**——按计划正文的 `head -200` 取到的全是
  `kcs/sound/*.mp3`，带宽受限，只差 10.6%；全量清单实为 49% mp3 + 49% png，随机抽样
  才反映真实收益。完整数据与两个踩坑记录（`cache_root` 必须先存在；临时配置的
  `workspace_root` 必须写绝对路径）见 003 正文的「实测结果」一节。

## 依赖说明

- 003、007、008 都改 `crates/emukc_cache/src/kache.rs` 或其下载层，而这些路径
  目前**零测试覆盖**。001 先建立 mock CDN 基线，后三者才有回归保护。
- 004、006、011 都改任务数据解析链，且都会改变 `.data/codex/quest.json` 的产物。
  002 先提交 fixture，后三者才能在不跑整轮网络 bootstrap 的前提下验证。
- 013 依赖 012：012 先把「资产里记录客户端版本 + 与实时版本比对」这条最小链路跑通，
  013 再决定要不要把四份版本记录收敛成一份。
- 003 与 009 都动 populate 体验，但改动点不重叠，可并行。
- 009 的步骤 4 依赖 007：007 只提供「404 与瞬时失败是两个错误值」这个能力，把
  populate 侧怎么用它留给了 009。两者之间原本有个缺口——**404 不进 pass 2 的内存
  重试**谁都没写——已在 2026-09-20 补进 009 步骤 4。

## 本次审计确认的关键事实（执行者可直接引用）

- `--greedy` 目前不做任何网络探测，产出与默认策略逐字节相同。`source/mod.rs:92`
  硬编码 `CacheListMakeStrategy::Rules`，`kcs2/mod.rs:22` 与
  `kcs2/resources/mod.rs:35` 各有一行 `let strategy = CacheListMakeStrategy::Manifest;`
  覆写调用方传入的策略。
- `.data/codex/quest.json` 当前含 8 条 `detail` 为字面量 `"_quest_id_NNN"` 的任务，
  12 条 `name` 为 `"n/a"` 的任务，以及 1 条（api_no 1033）`requirements` 为
  `{"And": []}` 的任务。前两者由 004 修复，后者由 006 修复。
- `crates/emukc_bootstrap/assets/resource_manifest.json` 是 9 个 decoder 资产中
  唯一不带 `scriptVersion` 字段的。
- `.sync-fingerprint.json` 记录的是 `6.3.0.0`，而资产已同步到 `6.3.5.0`。

### 2026-09-20 实测：rules 与 manifest 两种策略的清单差异

在 `82d2203` + 计划 003 的两行改动之上实测：

- `cache make-list --overwrite`（Default/Rules 策略）→ **73,050** 条
- `cache make-list --manifest` → **94,558** 条
- **rules 是 manifest 的严格子集**（rules 独有 0 条）；manifest 多出的 21,508 条
  全部落在 `kcs2/resources/ship`（20,327）和 `kcs2/resources/slot`（1,181）

对这两份清单各做随机抽样、跟随 301 重定向后实测 HTTP 状态：

| 抽样来源 | 样本量 | 200 | 404 |
|---|---|---|---|
| rules 清单 | 25 | 25 | 0 |
| manifest 独有部分 | 85 | 6 | 79 |

结论：**默认用 Rules 策略**。manifest 多出的那两万条里约 93% 是不存在的资源，
下载它们纯属浪费——这正是 `cache-manifest-integration.md` 里
「让 fallback 在 decoder 已覆盖的家族上展开会浪费下载、请求不存在的资源」
所描述的情况。

但反过来也有一个**尚未解决的发现**：manifest 独有部分里约 7%（估算 1,500 条
左右）是**真实存在**的，说明 decoder 规则对 ship/slot 变体家族的覆盖仍有缺口，
Rules 清单漏掉了这些资源。抽样中命中的类别包括 `banner_dmg`。
补齐它的正确做法不是复活 Greedy 的暴力枚举，而是把 manifest 差集当作候选集做
一次性存在性探测，把确实存在的并入规则——候选来自差集而非枚举，量级是两万次
探测而不是无边界搜索。这件事尚未立计划。

## 2026-09-20 计划外已落地的改动

以下改动不属于本计划集任何一份，但触及了它们的范围，执行者跑漂移检查时会看到：

- `populate.rs` 删除了逐项 spinner（每个文件一个 `mp.add()`）。indicatif 0.18 只回收
  `ordering` 头部连续的僵尸条，而头部是常驻的聚合条，所以 73k 个 spinner 一个都不会
  被释放，每次重绘都要遍历全量。全本地命中的一轮从 2m13s / 220s CPU 降到 3s / 1.25s。
  009 的基线已相应推进到 `9bd9f59` + 该未提交改动。
- cache 清单从 73,050 收敛到 73,031：`slot.rs` 的 `card_t` 补上了 `generate.rs` 早就
  在用的 `enemy_slot_border`(1500) 过滤，`EVENT_SHIP_HOLES` 补了 6299/6301/6303，
  新增 `ALBUM_STATUS_HOLES`（743/744/745/748/749，补给形态舰，`start2` 里无字段可判）。
  三者去掉的正是 2026-09-20 那轮 populate 全部 19 条 404。

补充一条给计划 010 的事实：`--greedy` 的 holes 报告不只是「产出与默认策略相同」——
`ship.rs` 的 `HOLES_COLLECTOR` 有读取方和清空方但**没有任何写入方**，所以
`holes_report.txt` 恒为空，`GreedyConfig.concurrent` 也没有消费者。
`z/cache/holes_report.txt` 是 2026-04-20 的遗物，不是当前数据。

## 已考虑并否决

- 拆分 `make_list/mod.rs`（1383 行）与 `manifest/generate.rs`（1872 行）：在 010
  决定 Greedy 去留之前拆分是白拆，拆完还要再拆一次。010 落地后可另行评估。
- 给 `fetch_from_remote` 加 per-CDN 熔断 / 健康度跟踪：观测到的失败是代理侧握手
  掉线，不是单个 CDN 主机故障；003 的两行改动直接消除成因。
- 用 `main-decoder` 的解码产物替代 kccp 作为任务名称/描述来源：
  `grep -c '_quest_id_' main-decoder/out/main.decoded.js` 为 0，解码产物里没有任何
  任务字符串，此路不通。
- 更换第三方数据源：数据本身从未缺失（`kccp_quests.json` 里 771 个 id 全在），
  问题在本仓库的解析器。先做 004，再谈换源。
- 让 populate 在「失败项全为 404」时 exit 0：退出码在本仓库没有自动化消费者，而
  「落盘 + 信任人去看」已有反例（空了五个月无人发现的 `holes_report.txt`）。
  完整理由记在 009 的「维护须知」。
