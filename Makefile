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

ifeq ($(PROFILE),release)
CARGO_PROFILE_FLAG := --release
else
CARGO_PROFILE_FLAG :=
endif

.DEFAULT_GOAL := help

.PHONY: help build run serve test clippy fmt bootstrap decode-main cache-make-list cache-populate clean-debug

help: ## 显示本帮助
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

build: ## 编译 workspace
	$(CARGO) build $(CARGO_PROFILE_FLAG)

run: ## 启动服务器 (等同 serve)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- serve

serve: ## 启动服务器 (等同 run)
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- serve

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

cache-make-list: ## 生成缓存资源清单
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- cache make-list --overwrite

cache-populate: ## 按清单填充缓存 (CONCURRENT=$(CONCURRENT))
	$(CARGO) run $(CARGO_PROFILE_FLAG) -- cache populate --concurrent $(CONCURRENT)

clean-debug: ## 只清理 debug 产物, 保留 release
	$(CARGO) clean --profile dev
