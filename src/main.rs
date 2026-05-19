use jane_mcp::common::{init_logging, Config};
use jane_mcp::news_api::NewsService;
use jane_mcp::weather_api::WeatherService;
use rmcp::{
    handler::server::router::Router,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ErrorData},
    schemars, tool, tool_handler, tool_router, ServerHandler,
    transport::streamable_http_server::{
        session::local::LocalSessionManager,
        StreamableHttpServerConfig, StreamableHttpService,
    },
};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(description = "Parameters for getting weather forecast")]
struct GetWeatherParams {
    #[schemars(description = "City name, e.g. 'Beijing', 'Shanghai', 'London'")]
    city: String,
    #[schemars(description = "Number of forecast days (1-7, default 3)")]
    days: Option<u8>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(description = "Parameters for searching news")]
struct GetNewsParams {
    #[schemars(description = "Search query or topic, e.g. 'technology', 'AI', 'economy'")]
    query: Option<String>,
    #[schemars(description = "Language code: 'zh' for Chinese, 'en' for English")]
    language: String,
    #[schemars(description = "Maximum number of results (1-10, default 5)")]
    max_results: Option<u8>,
    #[schemars(
        description = "News category: general, world, nation, business, technology, entertainment, sports, science, health"
    )]
    category: Option<String>,
}

#[derive(Clone)]
struct JaneMcpServer {
    weather_service: Arc<WeatherService>,
    news_service: Arc<NewsService>,
}

impl JaneMcpServer {
    fn new() -> Self {
        Self {
            weather_service: Arc::new(WeatherService::new()),
            news_service: Arc::new(NewsService::new()),
        }
    }
}

#[tool_router]
impl JaneMcpServer {
    #[tool(description = "Get weather forecast for a given city. Supports Chinese and English city names. Returns temperature, precipitation, wind speed, humidity and weather conditions for the next few days.")]
    async fn get_weather(
        &self,
        Parameters(params): Parameters<GetWeatherParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let days = params.days.unwrap_or(3);
        match self.weather_service.get_weather(&params.city, days).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result)])),
            Err(e) => {
                error!("Weather query failed: {}", e);
                Err(ErrorData::internal_error(
                    format!("Failed to get weather for '{}': {}", params.city, e),
                    None,
                ))
            }
        }
    }

    #[tool(description = "Search for news articles by query and language. Supports Chinese (zh) and English (en) news. Returns titles, summaries, sources and links.")]
    async fn get_news(
        &self,
        Parameters(params): Parameters<GetNewsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let max_results = params.max_results.unwrap_or(5);
        match self
            .news_service
            .get_news(params.query, &params.language, max_results, params.category)
            .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result)])),
            Err(e) => {
                error!("News query failed: {}", e);
                Err(ErrorData::internal_error(
                    format!("Failed to get news: {}", e),
                    None,
                ))
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for JaneMcpServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();

    let config = Config::load()?;
    let port = config.server.port;

    info!("Starting Jane MCP server on port {}", port);

    let http_service = StreamableHttpService::new(
        move || {
            let server = JaneMcpServer::new();
            let mut router = Router::new(server.clone());
            router.tool_router = JaneMcpServer::tool_router();
            Ok(router)
        },
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default()
            .with_allowed_hosts([
                format!("21.214.35.137:{}", port),
                format!("localhost:{}", port),
                format!("127.0.0.1:{}", port),
            ])
            .with_stateful_mode(false)
            .with_json_response(true),
    );

    let app = axum::Router::new().route("/", axum::routing::any_service(http_service));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("Jane MCP server listening on http://0.0.0.0:{}/", port);

    axum::serve(listener, app).await?;
    Ok(())
}
