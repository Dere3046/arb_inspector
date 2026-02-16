# arb_inspector

`arb_inspector` 是一个命令行工具，用于从高通 `xbl_config.img` 固件镜像中提取 OEM 元数据，包括主版本号、次版本号和防回滚版本号。它可进行 **ARB（防回滚）检测**，帮助识别防回滚保护级别。

[English](README.md)

## 功能特性

- 解析 ELF 格式的 `xbl_config.img` 文件
- 自动定位并读取包含 OEM 元数据的 HASH 段
- 输出 OEM 主版本、次版本和防回滚版本信息
- 轻量级，仅依赖 Rust 标准库，无需额外运行时

## 工作原理

1. **ELF 解析**  
   工具首先读取输入文件的 ELF 头部，验证其为有效的 64 位小端 ELF 文件，并获取程序头表的位置和数量。

2. **候选段收集**  
   遍历所有程序头，筛选出存在于文件中且大小合理的段作为候选段（HASH 段通常在其中）。

3. **HASH 段识别**  
   对每个候选段，以 4 字节对齐的方式在段起始附近扫描，寻找符合 HASH 段头部特征的数据结构（包含版本号和元数据区域大小）。

4. **OEM 元数据提取**  
   根据 HASH 头部中的偏移量，计算 OEM 元数据区域的起始位置，并读取三个 32 位整数：主版本、次版本和防回滚版本。

5. **结果输出**  
   将提取的三个值打印到控制台，便于查看。

## 使用方法

```bash
arb_inspector [--debug] [--block] <xbl_config.img>
```

- `<xbl_config.img>`：输入的固件镜像文件路径。
- `--debug`（或 `-d`）：可选标志，启用详细输出，显示扫描了哪些段以及为何选中某个段。
- `--block`（或 `-b`）：启用块设备模式。跳过文件大小边界检查，适用于直接从 Android 分区读取时 `metadata().len()` 可能返回错误值的情况。

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

`arb_inspector` 基于对已知固件样本的分析，使用启发式规则定位 HASH 段。虽然它在大多数设备上能正常工作，但**不能保证解析所有版本的 `xbl_config.img` 文件**，因为可能存在厂商特定的变体、加密或未来的格式更改。如果工具无法解析您的文件，请使用 `--debug` 标志运行，并在 GitHub 上提交问题，附上输出内容以及文件副本（如果允许）。

## 许可证

本项目采用 MIT 许可证。详情请参阅 [LICENSE](LICENSE) 文件。

## 联系方式与版权声明

如有任何问题或建议，请联系：**fine4trn@163.com**  
本工具仅供学习研究使用。如发现任何侵权行为，将按要求进行修改。
