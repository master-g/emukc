# EmuKC Makefile —— 统一功能入口
#
# 用法: make <target> [PROFILE=debug] [CONCURRENT=N]
#
# 前置条件:
#   - decode-main 需先安装 Bun 依赖 (cd main-decoder && bun install),
#     且已 bootstrap 出 z/cache/kcs2/js/main.js 与 z/cache/gadget_html5/js/kcs_const.js。

CARGO ?= cargo
# PROFILE: release | debug; 切换所有 cargo 目标的编译档位
PROFILE ?= release
# CONCURRENT: cache populate 并发数 (CLI 必填项的默认值)
CONCURRENT ?= 16
# SCENARIO: battle sim 预设场景 (fresh_1_1 | leveled_for_mid_boss)
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

.DEFAULT_GOAL := help

.PHONY: help build run serve serve-dump test clippy fmt bootstrap decode-main update drift-check drift-accept route-rules-sync route-rules-update route-oracle gear-bonus-sync gear-bonus-update gear-bonus-oracle kcnav-sync kcnav-normalize kcnav-update cache-make-list cache-populate battle-sim clean-debug

help: ## 显示本帮助
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

build: ## 编译 workspace
	$(CARGO) build $(CARGO_PROFILE_FLAG)

run: ## 启动服务器
	$(CARGO) run $(CARGO_PROFILE_FLAG)

serve: ## 启动服务器 (serve)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- serve

serve-dump: ## 启动服务器(auto 模式: 自动 auth + 开浏览器)并把 KCSAPI 请求/响应转储到 $(DUMP)
	EMUKC_KCSAPI_DUMP=$(DUMP) $(CARGO) run $(CARGO_PROFILE_FLAG)

test: ## 运行全部测试
	$(CARGO) test

clippy: ## 运行 clippy 检查
	$(CARGO) clippy --workspace

fmt: ## 格式化全部代码
	$(CARGO) fmt --all

bootstrap: ## 下载/刷新游戏数据 (--overwrite --force-update)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- bootstrap --overwrite --force-update

decode-main: ## decode main.js 并同步全部资源资产到 rust 项目
	cd main-decoder && bun run decode -- --sync-assets --sync-battle-assets --sync-resource-manifest

update: ## 全链更新游戏资源: bootstrap → 解码同步资产 → 漂移报告 → 装備ボーナス对拍 → 生成缓存清单
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- bootstrap --overwrite --force-update
	cd main-decoder && bun run decode -- --sync-assets --sync-battle-assets --sync-resource-manifest
	@echo "--- 资产漂移报告 (不阻断; review 过 diff 后跑 make drift-accept) ---"
	-$(CARGO) run $(CARGO_PROFILE_FLAG) -- battle drift-check
	@echo "--- 装備ボーナス对拍新客户端 (不阻断; 有差异则订正 main-decoder/gear-bonus-corrections.json) ---"
	-cd main-decoder && bun run gear-bonus-oracle
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- cache make-list --overwrite

drift-check: ## 比对已同步资产与基线, 有漂移则退出非零
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- battle drift-check

drift-accept: ## review 过 diff 之后, 把当前资产记为新基线
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- battle drift-check --accept

route-rules-sync: ## 下载钉住提交的羅針盤シミュ源码 (路由规则的来源) 到 .data/temp
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- route-rules sync

route-rules-update: route-rules-sync ## 从钉住的源码再生路由规则资产: 取源 → 解析 → 归一化 → 漂移报告
	cd main-decoder && bun run route-rules
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- route-rules normalize
	-$(CARGO) run $(CARGO_PROFILE_FLAG) -- battle drift-check

kcnav-sync: ## 从 KCNav 下载掉落与敌方编成的原始响应到 .data/temp/kcnav (单线程, 可续传; MAP=1-1 限定地图, INTERVAL=2 请求间隔秒)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- kcnav sync --interval $(or $(INTERVAL),2) $(if $(MAP),--map $(MAP),)

kcnav-normalize: ## 把已下载的 KCNav 响应归一化成一份文档 (不联网)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- kcnav normalize

kcnav-update: kcnav-sync kcnav-normalize drift-check ## 刷新 KCNav 数据: 下载 → 归一化 → 漂移报告

route-oracle: ## 用来源代码对拍已转换的路由规则 (需先 route-rules-update 并重建 codex), 报告写到 .data/temp
	cd main-decoder && bun run route-oracle

gear-bonus-sync: ## 下载钉住提交的 KC3Kai 文件 (装備ボーナス表的来源) 到 .data/temp
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- gear-bonus sync

gear-bonus-update: gear-bonus-sync ## 从钉住的源码再生装備ボーナス资产: 取源 → 转换 → 漂移报告 (之后重建 codex)
	cd main-decoder && bun run gear-bonus
	-$(CARGO) run $(CARGO_PROFILE_FLAG) -- battle drift-check

gear-bonus-oracle: ## 用客户端 main.js 的加成函数对拍装備ボーナス (需先 decode-main 并重建 codex), 报告写到 .data/temp
	cd main-decoder && bun run gear-bonus-oracle

cache-make-list: ## 生成缓存资源清单
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- cache make-list --overwrite

cache-populate: ## 按清单填充缓存 (CONCURRENT=$(CONCURRENT))
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- cache populate --concurrent $(CONCURRENT)

battle-sim: ## 跑 seeded 场景出击并打印战斗记录 (SCENARIO/SEED/FIND/MAX_SEEDS)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- battle sim --scenario $(SCENARIO) --seed $(SEED) $(BATTLE_SIM_FIND_FLAG)

clean-debug: ## 只清理 debug 产物, 保留 release
	$(CARGO) clean --profile dev
