# Openloader

Open source implementation of the blob running on the Cortex M0.

## Status
### Working
 - [X] PLL, very basic clocks
 - [X] UART
 - [X] Efuse
 - [X] DRAM init (tested on 32, 64 and 128 MB)
 - [X] USB
 - [X] Download protocol for stage 2

### TODO
 - [ ] NAND/NOR
 - [ ] microSD (probably)

### Not planned
- Pinctrl, non-basic clocks

## Credits
- [stefand](https://github.com/stefand) - lots of reverse engineering for this SoC; testing (64 MB)
- [Mio-sha512](https://github.com/Mio-sha512) - DRAM & USB & protocol drivers; testing (32 MB)
- [jschwartzenberg](https://github.com/jschwartzenberg) - testing (64MB)
- [exp-3](https://github.com/exp-3) - testing (32MB)

## 关于本fork修改的

增加SPI闪存初始化和更完备的USB/UART支持，用于与[Web Failsafe U-Boot](https://github.com/WifiChan/7520v3-u-boot)配套使用。

感谢Google Antigravity提供的Gemini 3.8 Flash与Gemini 4 Argon帮我实现了这个离谱的创意。

感谢[铸币](https://space.bilibili.com/35981144)与[小风时雨](https://github.com/yueluoying)两位朋友的测试与问题反馈。

以及感谢所有认真看完README并使用的各位用户......
