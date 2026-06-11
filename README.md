# Codeplag-rs

[![CI](https://github.com/qwq-298/Codeplag-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/qwq-298/Codeplag-rs/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/badge/coverage-76.92%25-green)](https://github.com/qwq-298/Codeplag-rs)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange)](https://www.rust-lang.org/)

> 基于 Rust 的多维度代码查重分析器，支持 7 种编程语言，采用 9 维指纹融合评分。

## 目录

- [功能特性](#功能特性)
- [架构概览](#架构概览)
- [安装](#安装)
- [使用指南](#使用指南)
- [查重技术详解](#查重技术详解)
- [加权评分公式](#加权评分公式)
- [支持的语言](#支持的语言)
- [配置参数](#配置参数)
- [项目结构](#项目结构)
- [测试](#测试)
- [CI/CD](#cicd)

## 功能特性

- **9 维指纹分析**：Winnowing 文本指纹 + AST 结构哈希 + 控制流图 + 调用图 + 定义-使用图 + 语句三元组 + Bag-of-Statements + 语义归一化 + Token 频率余弦相似度
- **7 语言支持**：Rust、Python、JavaScript、TypeScript、Go、C、C++、Java
- **6 种分析模式**：目录分析、文件对比、GitHub 仓库抓取、GitHub 搜索、项目级对比、批量对比
- **抗混淆能力**：变量重命名、代码格式化、注释增删、语句重排序、等价结构变换（for↔while、match↔if-else）
- **Chunk 匹配**：基于投票+偏移量对齐的代码块定位
- **函数级比对**：Tree-sitter 提取函数后逐个比对
- **指纹缓存**：SHA-256 内容寻址，避免重复计算
- **并行加速**：rayon 多线程比对
- **多格式输出**：文本 / JSON

## 架构概览

```
用户输入 → CLI 解析(clap) → 文件收集(GitHubFetcher)
                │
    ┌───────────┴───────────┐
    ▼                       ▼
fingerprint/winnowing    fingerprint/ast
· 注释剥离               · Tree-sitter AST 解析
· 空白归一化             · 结构哈希（排除标识符名）
· Token 化               · 语义归一化（for→while）
· k-gram 滑动窗口哈希    · CFG / CallGraph / DefUse
· Winnowing 稀疏采样     · Bag-of-Statements / Trigram
    │                       │
    └───────────┬───────────┘
                ▼
        CodeFingerprint (9 维)
                │
    ┌───────────┴───────────┐
    ▼                       ▼
FingerprintCache         SimilarityEngine
(磁盘 JSON 缓存)          · 加权融合 9 维分数
                         · Chunk 匹配（投票+偏移量）
                         · 阈值过滤
                         · 降序排序输出
```

## 安装

### 前置条件

- Rust 工具链（1.70+）
- Git（用于仓库抓取功能）

### 从源码构建

```bash
git clone https://github.com/qwq-298/Codeplag-rs.git
cd Codeplag-rs
cargo build --release
```

编译产物位于 `target/release/codeplag`（Linux/macOS）或 `target/release/codeplag.exe`（Windows）。

### GitHub API 搜索（可选）

如需使用 `search` 命令，设置环境变量：

```bash
export GITHUB_TOKEN=ghp_xxxxxxxxxxxx
```

## 使用指南

### 全局选项

所有子命令共享以下选项，**必须放在子命令之前**：

| 选项 | 默认值 | 说明 |
|------|--------|------|
| `-t, --threshold` | `0.5` | 相似度阈值 [0.0, 1.0]，低于此值不报告 |
| `--k-gram` | `5` | Winnowing 的 k-gram 大小 |
| `--window` | `4` | Winnowing 的滑动窗口大小 |
| `--functions` | — | 开启函数级比对模式 |
| `-v, --verbose` | — | 详细日志输出 |
| `--github-token` | 环境变量 | GitHub API 访问令牌 |

### `analyze` — 目录内全量分析

分析指定目录下所有源代码文件两两之间的相似度。

```bash
# 基本用法
codeplag analyze --path ./project_src

# JSON 输出
codeplag analyze --path ./project_src --output json

# 函数级比对
codeplag --functions analyze --path ./project_src

# 调整参数
codeplag --threshold 0.7 --k-gram 6 --window 5 analyze --path ./project_src
```

### `compare` — 单文件对比

将一个文件与另一个文件或整个目录对比。

```bash
# 文件 vs 文件
codeplag compare \
  --file test_fixtures/original/sort_rust.rs \
  --against test_fixtures/renamed/sort_rust.rs

# 文件 vs 目录
codeplag compare \
  --file suspect.rs \
  --against reference_project/

# 函数级对比
codeplag --functions compare \
  --file suspect.rs \
  --against reference_project/
```

### `project` — 项目级对比

比较两个完整项目目录的整体相似度（覆盖率感知评分）。

```bash
codeplag project \
  -a project_A/ \
  -b project_B/ \
  --output json
```

输出示例：
```json
{
  "project_score": 0.72,
  "file_matches": [
    {
      "file_a": "sort_rust.rs",
      "file_b": "my_sort_rust.rs",
      "similarity_score": 0.95,
      "winnowing_score": 0.91,
      "ast_score": 0.88
    }
  ]
}
```

### `fetch` — 抓取 GitHub 仓库

克隆并分析单个 GitHub 仓库内部的文件相似度。

```bash
codeplag fetch --repo https://github.com/user/repo.git
```

### `batch` — 批量抓取并两两对比

同时抓取多个 GitHub 仓库，对所有仓库对进行项目级对比。

```bash
codeplag batch \
  --repos https://github.com/user/repo1,https://github.com/user/repo2,https://github.com/user/repo3
```

输出按项目相似度降序排列。

### `search` — GitHub 代码搜索

通过 GitHub Code Search API 搜索与本地代码相似的公开仓库。

```bash
codeplag search \
  --path ./my_project/ \
  --limit 10
```

## 查重技术详解

本项目采用 **9 个独立维度** 的指纹分析，融合评分以应对多种代码混淆手段。

### 一、文本指纹层（Winnowing）

| 步骤 | 说明 |
|------|------|
| ① 注释剥离 | 移除 `//` 行注释和 `/* */` 块注释 |
| ② 空白归一化 | 压缩连续空白、去除括号/分号周围空格 |
| ③ Token 化 | 语言无关的词法分析，6 种 Token：Keyword / Identifier / Number / String / Operator / Punctuation |
| ④ k-gram 哈希 | 滑动 k 个 Token 的窗口，SHA-256 哈希（**标识符替换为 0xFF 占位符**，抗变量重命名） |
| ⑤ Winnowing 挑选 | 每个大小为 w 的窗口中选取**最小哈希值**作为指纹点（Schleimer-Wilerson 算法） |
| ⑥ Jaccard 相似度 | 指纹点集合的交集/并集 |

### 二、AST 结构分析层（Tree-sitter）

| 维度 | 实现 | 抗混淆能力 |
|------|------|-----------|
| **AST 结构哈希** | 节点类型 + 子节点类型序列 → SHA-256，**排除标识符名称** | 变量/函数重命名 |
| **语义归一化** | `for i in 0..n` 重哈希为等价 `while i < n`；布尔 `match` → `if-else` | 等价结构变换 |
| **Bag-of-Statements** | 块内语句结构哈希**排序后**再哈希 | 语句重排序 |
| **控制流图 (CFG)** | 提取 if/for/while/return/match 节点骨架哈希 | 变量重命名、格式变化 |
| **调用图** | caller→callee 边哈希，函数名不参与 | 函数重命名、内联/拆分 |
| **定义-使用图** | 变量 `def→use` 数据流边，位置归一化（乘以大质数） | 变量重命名 |
| **语句三元组** | 11 类语句的连续三元组类型序列 | 部分重排序 |

### 三、辅助维度

| 维度 | 方法 |
|------|------|
| **Token 频率余弦相似度** | 6 维 Token 类型频率向量的余弦夹角 |

### 语义归一化示例

```
原始代码:                         归一化后等价于:
for i in 0..n { body }    →    while i < n { body; i += 1 }

match cond {               →    if cond { A } else { B }
    true  => A,
    false => B
}
```

归一化哈希被**追加**到原始 AST 哈希集中，确保同一种代码的两种写法产生重叠指纹。

## 加权评分公式

### 文件级 / 函数级（有 AST 哈希时）

```
score = 0.35 × Winnowing + 0.20 × AST 结构
      + 0.10 × Bag-AST     + 0.05 × Token  余弦
      + 0.10 × CFG          + 0.05 × CallGraph
      + 0.05 × DefUse       + 0.10 × StmtTrigram
```

### 文件级 / 函数级（无 AST 哈希时，解析失败降级）

```
score = 0.40 × Winnowing + 0.20 × Bag-AST
      + 0.05 × Token 余弦  + 0.10 × CFG
      + 0.10 × CallGraph    + 0.05 × DefUse
      + 0.10 × StmtTrigram
```

### 项目级（有 AST 哈希时，权重不同于文件级）

```
score = 0.22 × Winnowing + 0.18 × AST 结构
      + 0.10 × Bag-AST     + 0.10 × Token 余弦
      + 0.12 × CFG          + 0.10 × CallGraph
      + 0.10 × DefUse       + 0.08 × StmtTrigram
```

### 项目总分（Coverage-aware）

```
project_score = Σ(每个文件最佳匹配分) / max(|项目A|, |项目B|)
```

防止文件少的一方虚高评分。例如 A 有 2 文件全匹配，B 有 4 文件 → 项目分 = 2/4 = 0.5。

## 支持的语言

| 语言 | 扩展名 | Tree-sitter 语法 |
|------|--------|-----------------|
| Rust | `.rs` | `tree-sitter-rust` |
| Python | `.py` | `tree-sitter-python` |
| JavaScript | `.js` | `tree-sitter-javascript` |
| TypeScript | `.ts` | `tree-sitter-javascript` |
| Go | `.go` | `tree-sitter-go` |
| C | `.c`, `.h` | `tree-sitter-c` |
| C++ | `.cpp`, `.cc`, `.cxx`, `.hpp` | `tree-sitter-cpp` |
| Java | `.java` | `tree-sitter-java` |

不同语言的文件**不会相互比对**。

## 配置参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `k_gram_size` | 5 | Winnowing k-gram 中的 Token 数量 |
| `window_size` | 4 | Winnowing 滑动窗口大小 |
| `threshold` | 0.5 | 相似度阈值，低于此值的配对不会出现在结果中 |
| `min_file_size` | 100 bytes | 小于此值的文件跳过 |
| `max_file_size` | 1,000,000 bytes | 大于此值的文件跳过 |

## 项目结构

```
Codeplag-rs/
├── src/
│   ├── main.rs                # CLI 入口，6 条命令分发
│   ├── lib.rs                 # 模块声明
│   ├── cli/mod.rs             # clap CLI 参数定义
│   ├── core/types.rs          # 全部数据结构（Language, CodeFingerprint 等）
│   ├── engine/mod.rs          # 核心引擎 + 指纹缓存 + Chunk 匹配
│   ├── fetcher/
│   │   ├── mod.rs
│   │   └── github.rs          # GitHub 仓库抓取 + 本地文件收集
│   └── fingerprint/
│       ├── mod.rs
│       ├── winnowing.rs       # Winnowing 文本指纹层
│       └── ast.rs             # AST 结构分析（8 种维度）
├── tests/
│   ├── integration_test.rs    # 端到端集成测试（20 个）
│   └── cli_test.rs            # CLI 命令测试（19 个）
├── test_fixtures/             # 测试数据（4 类 × 7 语言 = 28 份代码）
│   ├── original/              # 原始排序算法实现
│   ├── renamed/               # 变量重命名 + 注释混淆
│   ├── restructured/          # 等价算法不同实现
│   └── unrelated/             # 无关代码
├── testcases/                 # 额外测试用例（16 组 Rust 代码）
├── .github/workflows/ci.yml   # CI 流水线（Build/Test + Clippy + Coverage）
├── .clippy.toml               # Clippy 配置
├── rustfmt.toml               # 代码格式配置
└── Cargo.toml                 # 项目元数据与依赖
```

## 测试

```bash
# 运行全部测试（112 个）
cargo test

# 仅运行单元测试（73 个）
cargo test --lib

# 运行集成测试
cargo test --test integration_test

# 运行 CLI 测试
cargo test --test cli_test

# 生成覆盖率报告（需安装 cargo-tarpaulin）
cargo tarpaulin --out Html --output-dir coverage
```

### 测试统计

| 类型 | 数量 | 说明 |
|------|------|------|
| 单元测试 | 73 | 覆盖 types / winnowing / AST / fetcher |
| CLI 集成测试 | 19 | 覆盖所有 6 条命令 + 错误处理 |
| 端到端集成测试 | 20 | 覆盖 9 维指纹维度 + 跨语言 + 项目级评比 |
| **总计** | **112** | |

### 代码覆盖率

| 模块 | 覆盖率 |
|------|--------|
| `fingerprint/winnowing.rs` | 97.9% |
| `core/types.rs` | 95.5% |
| `engine/mod.rs` | 91.7% |
| `fingerprint/ast.rs` | 89.8% |
| `fetcher/github.rs` | 64.1% |
| `main.rs` | 37.4% |
| **整体** | **76.92%** |

## CI/CD

GitHub Actions 自动运行三阶段流水线：

1. **Build & Test**：Ubuntu + Windows 双平台构建与测试
2. **Static Analysis**：`cargo fmt --check` + `cargo clippy -D warnings`
3. **Code Coverage**：`cargo-tarpaulin` 生成覆盖率报告并上传 Codecov

## 许可证

MIT
