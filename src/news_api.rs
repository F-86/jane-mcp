use crate::common::{HttpClient, TtlCache};
use anyhow::{Context, Result};
use std::time::Duration;
use tracing::{debug, info};

#[derive(Clone)]
pub struct NewsService {
    client: HttpClient,
    cache: TtlCache<String>,
}

impl NewsService {
    pub fn new() -> Self {
        Self {
            client: HttpClient::new().expect("Failed to create HTTP client"),
            cache: TtlCache::new(Duration::from_secs(300)),
        }
    }

    pub async fn get_news(
        &self,
        query: Option<String>,
        language: &str,
        max_results: u8,
        _category: Option<String>,
    ) -> Result<String> {
        let max_results = max_results.clamp(1, 10);
        let (lang_code, gl, ceid) = normalize_language(language);
        let q = query.as_deref().unwrap_or("latest");
        let cache_key = format!("{}-{}-{}", q, lang_code, max_results);

        if let Some(cached) = self.cache.get(&cache_key).await {
            debug!("News cache hit for {}", q);
            return Ok(cached);
        }

        let articles = self.fetch_rss(q, &lang_code, &gl, &ceid, max_results).await?;
        let markdown = self.format_news(q, &articles);

        self.cache.set(cache_key, markdown.clone()).await;
        Ok(markdown)
    }

    async fn fetch_rss(
        &self,
        query: &str,
        hl: &str,
        gl: &str,
        ceid: &str,
        max_results: u8,
    ) -> Result<Vec<RssArticle>> {
        let url = format!(
            "https://news.google.com/rss/search?q={}&hl={}&gl={}&ceid={}",
            urlencoding::encode(query),
            hl,
            gl,
            ceid
        );

        info!("Fetching news RSS: query={}, lang={}", query, hl);

        let bytes = self
            .client
            .inner()
            .get(&url)
            .send()
            .await
            .context("News RSS request failed")?
            .bytes()
            .await
            .context("Failed to read news RSS body")?;

        let channel = rss::Channel::read_from(&bytes[..])
            .context("Failed to parse news RSS")?;

        let articles: Vec<RssArticle> = channel
            .items()
            .iter()
            .take(max_results as usize)
            .map(|item| RssArticle {
                title: item.title().unwrap_or("无标题").to_string(),
                link: item.link().unwrap_or("").to_string(),
                description: item.description().map(|s| s.to_string()),
                pub_date: item.pub_date().map(|s| s.to_string()),
                source: item.source().and_then(|s| s.title().map(|t| t.to_string())),
            })
            .collect();

        Ok(articles)
    }

    fn format_news(&self, query: &str, articles: &[RssArticle]) -> String {
        if articles.is_empty() {
            return format!("未找到关于 '{}' 的新闻。\n", query);
        }

        let mut md = format!("## 新闻搜索结果: '{}'\n\n", query);
        for (i, article) in articles.iter().enumerate() {
            let desc = article.description.as_deref().unwrap_or("暂无摘要");
            let source = article.source.as_deref().unwrap_or("Google News");
            let published = article.pub_date.as_deref().unwrap_or("未知时间");
            md.push_str(&format!(
                "### {}. {}\n- **来源**: {}\n- **发布时间**: {}\n- **摘要**: {}\n- **链接**: {}\n\n",
                i + 1,
                article.title,
                source,
                published,
                desc,
                article.link
            ));
        }
        md
    }
}

#[derive(Debug, Clone)]
struct RssArticle {
    title: String,
    link: String,
    description: Option<String>,
    pub_date: Option<String>,
    source: Option<String>,
}

fn normalize_language(lang: &str) -> (String, String, String) {
    match lang.to_lowercase().as_str() {
        "zh" | "zh-cn" | "中文" | "chinese" => {
            ("zh-CN".to_string(), "CN".to_string(), "CN:zh-Hans".to_string())
        }
        "en" | "english" | "英文" => {
            ("en-US".to_string(), "US".to_string(), "US:en".to_string())
        }
        _ => {
            ("en-US".to_string(), "US".to_string(), "US:en".to_string())
        }
    }
}
