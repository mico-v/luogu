.PHONY: help build install uninstall fetch judge catalog clean

help:
	@echo "Luogu CLI - 洛谷本地竞赛练习工具"
	@echo ""
	@echo "🚀 快速命令:"
	@echo "  make build              构建二进制文件"
	@echo "  make install            安装到系统 (需要 sudo)"
	@echo "  make uninstall          从系统卸载"
	@echo ""
	@echo "📝 在项目目录中使用:"
	@echo "  make run-fetch P=P1000        获取问题 P1000"
	@echo "  make run-judge P=P1000        评测问题 P1000 (自动检测语言)"
	@echo "  make run-judge-python P=P1000 评测 Python 版本"
	@echo "  make run-judge-opt P=P1000    O3 优化级别"
	@echo "  make run-judge-cpp20 P=P1000  C++20 标准"
	@echo "  make run-catalog              列出所有问题"
	@echo "  make run-catalog-history      查看评测历史"
	@echo ""
	@echo "🛠️  开发命令:"
	@echo "  make clean              清理构建产物"
	@echo ""

build:
	@cargo build --release

install: build
	@cargo install --path .
	@echo "✓ 已安装到系统！现在可以直接运行: luogu fetch <pid>"

uninstall:
	@cargo uninstall luogu
	@echo "✓ 已从系统卸载"

run-fetch:
	@test -n "$(P)" || (echo "用法: make run-fetch P=P1000"; exit 1)
	@cargo run --release -- fetch $(P)

run-judge:
	@test -n "$(P)" || (echo "用法: make run-judge P=P1000"; exit 1)
	@cargo run --release -- judge $(P)

run-judge-python:
	@test -n "$(P)" || (echo "用法: make run-judge-python P=P1000"; exit 1)
	@cargo run --release -- judge $(P) --language python

run-judge-opt:
	@test -n "$(P)" || (echo "用法: make run-judge-opt P=P1000"; exit 1)
	@cargo run --release -- judge $(P) --opt O3

run-judge-cpp20:
	@test -n "$(P)" || (echo "用法: make run-judge-cpp20 P=P1000"; exit 1)
	@cargo run --release -- judge $(P) --std c++20 --opt O3

run-catalog:
	@cargo run --release -- catalog

run-catalog-history:
	@cargo run --release -- catalog --history

clean:
	@cargo clean
	@rm -f Cargo.lock
