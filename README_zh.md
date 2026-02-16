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
arb_inspector [--debug] <xbl_config.img>
```

- `<xbl_config.img>`：输入的固件镜像文件路径。  
- `--debug`（或 `-d`）：可选参数，启用详细输出，显示扫描了哪些段以及最终选中某个段的原因。

### 示例

```bash
$ arb_inspector xbl_config.img
OEM Metadata from xbl_config.img:
  Major Version         : 3
  Minor Version         : 0
  Anti-Rollback Version : 0

$ arb_inspector --debug xbl_config.img
[DEBUG] Scanning segment 0 at file offset 0x0 (size 0x1c8)
[DEBUG] Scanning segment 6 at file offset 0x5d000 (size 0x130c)
[DEBUG] Segment at file offset 0x5d000: possible header at offset +0x4 (file 0x5d004)
[DEBUG]  -> OEM at +0x40 (file 0x5d040): major=3, minor=0, arb=0
[DEBUG] >>> SELECTED segment 6 (offset 0x5d000) with header at +0x4
OEM Metadata from xbl_config.img:
  Major Version         : 3
  Minor Version         : 0
  Anti-Rollback Version : 0
```

## 局限性

`arb_inspector` 基于对已知固件样本的分析，通过启发式规则定位 HASH 段。虽然它在大多数设备上能够正常工作，但**无法保证能解析所有版本的 `xbl_config.img` 文件**，因为不同厂商可能有定制格式，或未来格式可能发生变化。如果工具解析失败，请使用 `--debug` 参数运行，并在 GitHub 提交 Issue，附上调试输出和固件文件（如允许）。

## 许可证

本项目采用 MIT 许可证。详情参见 [LICENSE](LICENSE) 文件。

## 联系方式与侵权声明

如有任何问题或建议，请联系：**fine4trn@163.com**  
本工具仅用于学习与研究，若涉及侵权，将按要求做出相应修改。