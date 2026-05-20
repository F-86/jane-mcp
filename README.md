# Jane MCP

基于 SSE 传输的 MCP Server，同时提供天气查询和新闻查询能力。

## 功能

- **天气查询**: 按城市名查询当前及未来天气
  - 暴露工具: `get_weather(city, days)`
  - 数据源: [Open-Meteo](https://open-meteo.com/) (免费，无需 API Key)

- **新闻查询**: 按关键词/语言搜索新闻
  - 暴露工具: `get_news(query, language, max_results, category)`
  - 数据源: Google News RSS (免费，无需 API Key)

- **服务端口**: `8080`

## 快速开始

### 1. 构建

```bash
cargo build --release
```

### 2. 通过 systemd 运行

```bash
# 复制 service 文件到系统目录
sudo cp jane-mcp.service /etc/systemd/system/

# 重载 systemd 配置
sudo systemctl daemon-reload

# 启动服务
sudo systemctl start jane-mcp

# 设置开机自启
sudo systemctl enable jane-mcp
```

常用命令：

```bash
# 查看状态
sudo systemctl status jane-mcp

# 查看日志
sudo journalctl -u jane-mcp -f

# 停止服务
sudo systemctl stop jane-mcp

# 重启服务
sudo systemctl restart jane-mcp
```

### 3. MCP Client 配置

#### Claude Desktop

在 `claude_desktop_config.json` 中添加:

```json
{
  "mcpServers": {
    "jane": {
      "transport": "sse",
      "url": "http://localhost:8080/mcp"
    }
  }
}
```

#### Cursor

在 Cursor Settings -> MCP Servers 中添加:

- Name: `jane`, URL: `http://localhost:8080/mcp`

## 工具说明

### get_weather

查询指定城市的天气预报。

参数:
- `city` (string, required): 城市名，支持中文和英文，如 "Beijing", "Shanghai", "London"
- `days` (number, optional): 预报天数，范围 1-7，默认 3

返回: Markdown 格式的天气预报，包含每日温度、降水、风速、湿度、天气状况、日出日落时间。

### get_news

搜索新闻文章。

参数:
- `query` (string, optional): 搜索关键词或主题，如 "technology", "AI"
- `language` (string, required): 语言代码，`zh` 中文，`en` 英文
- `max_results` (number, optional): 最大返回数量 1-10，默认 5
- `category` (string, optional): 新闻分类，可选值: general, world, nation, business, technology, entertainment, sports, science, health

返回: Markdown 格式的新闻列表，包含标题、来源、发布时间、摘要和链接。

## 技术栈

- Rust 2024
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - 官方 MCP Rust SDK
- [Open-Meteo API](https://open-meteo.com/) - 免费天气数据
- Google News RSS - 免费新闻数据

## 许可证

MIT
