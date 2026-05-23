# OpenUtau 声库管理器

**概述: ** 可一键下载安装在线声库，只要是openutau支持的都可以，声库包文件必须是.zip。使用Vue3 + Naive UI + Rust + Tauri 实现。

**解压和下载方法实现：** [7-Zip](https://www.7-zip.org) 执行声库解压，[Aria2](https://aria2.github.io) 执行声库下载，所需组件已内置到 **3rd** 文件夹，也可自行指定（需要复制完整路径）。


**声库源api实现参考（请打开美观输出）：** [https://res.ai-lab.top/api/voicebanks.json](https://res.ai-lab.top/api/voicebanks.json)
# 参考资料
**代码编写辅助：** Gemini 3 Flash

**界面参考：** [UtauV](https://github.com/emeraldsingers/UtauV) 的包管理器

**声库API参考：** [https://github.com/emeraldsingers/UtauV_Packages](https://github.com/emeraldsingers/UtauV_Packages)

# 开发与构建
```bash
# 安装依赖
npm install
# 启动开发环境
npx tauri dev
# 打包应用
npx tauri build