# arb_inspector

[English](README.md) | [中文](README_zh.md)

`arb_inspector` 是一个命令行工具，用于从 Qualcomm 设备的 `xbl_config.img` 固件镜像中提取 OEM 元数据，包括主版本号、次版本号和防回滚版本（Anti‑Rollback Version）。

## 功能特性

- 解析 ELF 格式的 `xbl_config.img` 文件  
- 自动定位并读取 HASH 段中的 OEM 元数据  
- 输出 OEM Major、Minor 和 Anti‑Rollback 版本信息  
- 轻量级，仅依赖 Rust 标准库，无需额外运行时  

## 工作原理

1. **ELF 解析**  
   工具首先读取输入文件的 ELF 头部，验证其为合法的 64 位小端 ELF 格式，并获取程序头表的位置和数量。

2. **候选段收集**  
   遍历所有程序头，筛选出文件内存在且大小合理的段作为候选段（HASH 段通常位于这些段中）。

3. **HASH 段识别**  
   对每个候选段，在起始位置附近按 4 字节对齐扫描，寻找符合 HASH 段头部特征的数据结构（包含版本号、各元数据区域大小等信息）。

4. **OEM 元数据提取**  
   根据 HASH 头部的偏移计算出 OEM 元数据区域的起始位置，从中读取三个 32 位整数：Major、Minor 和 Anti‑Rollback 版本。

5. **结果输出**  
   将提取到的三个值打印到控制台，便于用户查看。

## 使用方法

```bash
arb_inspector <xbl_config.img>
```

- `<xbl_config.img>`：输入的固件镜像文件路径。

### 示例

```bash
$ arb_inspector xbl_config.img
OEM Metadata from xbl_config.img:
  Major Version         : 3
  Minor Version         : 0
  Anti-Rollback Version : 0
```

## 关于解析覆盖率

`arb_inspector` 基于对已知固件样本的分析，通过启发式规则定位 HASH 段并提取 OEM 元数据。虽然它在大多数设备上能够正常工作，但**无法保证能解析所有版本的 `xbl_config.img` 文件**，原因如下：

- 固件格式可能随厂商更新而变化
- 某些定制固件可能采用非标准的段结构或加密/压缩
- 启发式规则需要平衡准确率和覆盖率，可能漏掉部分变体

## 如果工具无法解析您的文件

如果您遇到了解析失败的情况，欢迎在 GitHub 仓库提交 **Issue**，并提供以下信息以便改进：

- 设备型号与固件来源
- 附上 `xbl_config.img` 文件（如允许）
- 预期的 ARB 值（如果有）
- 工具运行时的完整输出

我们会根据反馈不断优化规则，让工具支持更多固件版本。感谢您的理解与支持！

## 许可证

本项目采用 MIT 许可证。详情参见 [LICENSE](LICENSE) 文件。

## 联系方式与侵权声明

如有任何问题或建议，请联系：**fine4trn@163.com**  
本工具仅用于学习与研究，若涉及侵权，将按要求做出相应修改。
