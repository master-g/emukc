# EmuKC Makefile —— 统一功能入口
#
# 用法: make <target> [PROFILE=debug] [CONCURRENT=N]
#
# 前置条件:
#   - decode-main 需先安装 Bun 依赖 (cd main-decoder && bun install),
#     且已 bootstrap 出 <cache_root>/kcs2/js/main.js 与 <cache_root>/gadget_html5/js/kcs_const.js。

CARGO ?= cargo
# PROFILE: release | debug; 切换所有 cargo 目标的编译档位
PROFILE ?= release
# CONCURRENT: cache populate 并发数
CONCURRENT ?= 16
# SCENARIO: battle sim 的预设场景 (crates/emukc_gameplay/src/scenario/mod.rs), 或 headless-check 的场景 (tests/headless/run.py)
SCENARIO ?= fresh_1_1
# SEED: battle sim RNG 种子 (同种子 + 场景可复现整场出击)
SEED ?= 1
# FIND: 可选, 搜索命中某分支的种子 (night | cutin); 留空则只跑单次
FIND ?=
# MAX_SEEDS: --find 搜索时最多尝试的种子数
MAX_SEEDS ?= 1000
# DUMP: serve-dump 的 KCSAPI 请求/响应转储输出文件 (JSONL)
DUMP ?= .data/logs/kcsapi_dump.jsonl

ifeq ($(PROFILE),release)
CARGO_PROFILE_FLAG := --release
else
CARGO_PROFILE_FLAG :=
endif

ifeq ($(FIND),)
BATTLE_SIM_FIND_FLAG :=
else
BATTLE_SIM_FIND_FLAG := --find $(FIND) --max-seeds $(MAX_SEEDS)
endif

# 子目标递归调用时不打印 Entering/Leaving directory
MAKEFLAGS += --no-print-directory

.DEFAULT_GOAL := help

.PHONY: help build serve serve-dump test clippy fmt bootstrap decode-main update update-all drift-check drift-accept route-rules-update route-oracle gear-bonus-update gear-bonus-oracle remodel-tables-oracle kcnav-sync kcnav-normalize cache-make-list cache-populate cache-list-oracle battle-sim headless-check clean-debug

help: ## 显示本帮助
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

build: ## 编译 workspace
	$(CARGO) build $(CARGO_PROFILE_FLAG)

serve: ## 启动服务器 (serve)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- serve

serve-dump: ## 启动服务器(auto 模式: 自动 auth + 开浏览器)并把 KCSAPI 请求/响应转储到 $(DUMP)
	EMUKC_KCSAPI_DUMP=$(DUMP) $(CARGO) run $(CARGO_PROFILE_FLAG)

test: ## 运行全部测试
	$(CARGO) test --workspace

clippy: ## 运行 clippy 检查
	$(CARGO) clippy --workspace --all-targets -- -D warnings

fmt: ## 格式化全部代码
	$(CARGO) fmt --all

bootstrap: ## 下载/刷新游戏数据 (--overwrite --force-update)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- bootstrap --overwrite --force-update

decode-main: ## decode main.js 并同步全部资源资产到 rust 项目
	cd main-decoder && bun run decode -- --sync-assets --sync-battle-assets --sync-resource-manifest

update: ## 全链更新游戏资源: bootstrap → 解码同步资产 → 重建 codex → 漂移报告 → 改装表/装備ボーナス对拍 → 生成缓存清单
	$(MAKE) bootstrap
	$(MAKE) decode-main
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- bootstrap --codex-only
	@echo "--- 资产漂移报告 (不阻断; review 过 diff 后跑 make drift-accept) ---"
	-$(MAKE) drift-check
	@echo "--- 改装表对拍新客户端 (不阻断; 有差异则修 main-decoder/src/remodel-tables.ts) ---"
	-$(MAKE) remodel-tables-oracle
	@echo "--- 装備ボーナス对拍新客户端 (不阻断; 有差异则订正 main-decoder/gear-bonus-corrections.json) ---"
	-$(MAKE) gear-bonus-oracle
	@echo "--- 缓存清单对拍新客户端 (不阻断; 对的是现有清单, 报出的目录先 cache-make-list 再看) ---"
	-$(MAKE) cache-list-oracle
	$(MAKE) cache-make-list

update-all: ## update 之后按新清单填充缓存: 只下载缺的与版本变了的文件 (CONCURRENT=$(CONCURRENT))
	$(MAKE) update
	$(MAKE) cache-populate

drift-check: ## 比对已同步资产与基线, 有漂移则退出非零
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- battle drift-check

drift-accept: ## review 过 diff 之后, 把当前资产记为新基线
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- battle drift-check --accept

route-rules-update: ## 从钉住的羅針盤シミュ源码再生路由规则资产: 取源到 .data/temp → 解析 → 归一化 → 漂移报告
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- route-rules sync
	cd main-decoder && bun run route-rules
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- route-rules normalize
	-$(MAKE) drift-check

kcnav-sync: ## 从 KCNav 下载掉落与敌方编成的原始响应到 .data/temp/kcnav (单线程, 可续传; MAP=1-1 限定地图, INTERVAL=2 请求间隔秒)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- kcnav sync --interval $(or $(INTERVAL),2) $(if $(MAP),--map $(MAP),)

kcnav-normalize: ## 把已下载的 KCNav 响应归一化成一份文档 (不联网)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- kcnav normalize

route-oracle: ## 用来源代码对拍已转换的路由规则 (需先 route-rules-update 并重建 codex), 报告写到 .data/temp
	cd main-decoder && bun run route-oracle

gear-bonus-update: ## 从钉住的 KC3Kai 文件再生装備ボーナス资产: 取源到 .data/temp → 转换 → 漂移报告 (之后重建 codex)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- gear-bonus sync
	cd main-decoder && bun run gear-bonus
	-$(MAKE) drift-check

gear-bonus-oracle: ## 用客户端 main.js 的加成函数对拍装備ボーナス (需先 decode-main 并重建 codex), 报告写到 .data/temp
	cd main-decoder && bun run gear-bonus-oracle

remodel-tables-oracle: ## 用客户端 main.js 的函数对拍改装用的開発資材/高速建造材等表 (需先 decode-main)
	cd main-decoder && bun run remodel-tables-oracle

cache-make-list: ## 生成缓存资源清单
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- cache make-list --overwrite

cache-populate: ## 按清单填充缓存 (CONCURRENT=$(CONCURRENT))
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- cache populate --concurrent $(CONCURRENT)

cache-list-oracle: ## 用客户端 main.js 对拍缓存清单: 客户端点名的目录是否都有条目、舰船与装备地址的后缀是否一致 (需先 decode-main 与 cache-make-list), 报告写到 .data/temp
	cd main-decoder && bun run cache-list-oracle

battle-sim: ## 跑 seeded 场景出击并打印战斗记录 (SCENARIO/SEED/FIND/MAX_SEEDS)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- battle sim --scenario $(SCENARIO) --seed $(SEED) $(BATTLE_SIM_FIND_FLAG)

HEADLESS_PYTHON ?= $(HOME)/.local/pipx/venvs/playwright/bin/python
headless-check: ## 无头浏览器里用真实客户端跑一个场景 (SCENARIO=transport_5_6; 需资源缓存、已解码的 main.js、Playwright+Chrome), 报告写到 .data/temp/headless
	$(CARGO) build
	$(HEADLESS_PYTHON) tests/headless/run.py $(SCENARIO)

clean-debug: ## 只清理 debug 产物, 保留 release
	$(CARGO) clean --profile dev
