use crate::gpt_service::ChatMessage;
use crate::gpt_service::ChatMessageRole::{System, User};
use crate::{gpt_service, AppError, GptParameters};
use reqwest::header::CONTENT_TYPE;
use reqwest::Client;
use std::time::Duration;
use thiserror::Error;

const ARTICLE_FETCH_TIMEOUT: Duration = Duration::from_secs(30);
const ARTICLE_TEXT_WRAP_WIDTH: usize = 120;
const MIN_ARTICLE_TEXT_LEN: usize = 1000;
const ARTICLE_SUMMARY_SYSTEM_CONTEXT: &str = "Проанализируй статью и дай краткое содержание. Применяй юмор в анализе. Ответ должен быть структурированным, разбитым на пункты и содержать максимум 300 симвалов.";

/// Why an article could not be summarized. Callers decide per variant whether
/// that is worth an error, a silent skip, or a fallback reply.
#[derive(Debug, Error)]
pub enum ArticleSummaryError {
    #[error("failed to fetch article: {0}")]
    Fetch(#[from] reqwest::Error),

    #[error("article is not HTML (content type: {0})")]
    NotHtml(String),

    #[error("failed to parse article HTML: {0}")]
    Parse(String),

    #[error("article text too short to summarize ({0} bytes)")]
    TooShort(usize),

    #[error("summary generation failed: {0}")]
    Llm(AppError),
}

pub async fn summarize_article(
    params: &GptParameters,
    url: &str,
) -> Result<String, ArticleSummaryError> {
    let text = fetch_article_text(&params.http_client, url).await?;
    summarize_text(params, url, text)
        .await
        .map_err(ArticleSummaryError::Llm)
}

/// Fetch `url` and reduce it to plain text, rejecting anything that is not a
/// successful HTML page with enough text to be worth an LLM call.
pub async fn fetch_article_text(client: &Client, url: &str) -> Result<String, ArticleSummaryError> {
    let response = client
        .get(url)
        .timeout(ARTICLE_FETCH_TIMEOUT)
        .send()
        .await?
        .error_for_status()?;
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    if !is_html_content_type(&content_type) {
        return Err(ArticleSummaryError::NotHtml(content_type));
    }

    let html = response.text().await?;
    let text = html2text::from_read(html.as_bytes(), ARTICLE_TEXT_WRAP_WIDTH)
        .map_err(|err| ArticleSummaryError::Parse(err.to_string()))?;
    if text.len() < MIN_ARTICLE_TEXT_LEN {
        return Err(ArticleSummaryError::TooShort(text.len()));
    }
    Ok(text)
}

async fn summarize_text(
    params: &GptParameters,
    url: &str,
    text: String,
) -> Result<String, AppError> {
    let context = vec![
        ChatMessage {
            role: System,
            content: ARTICLE_SUMMARY_SYSTEM_CONTEXT.to_string(),
        },
        ChatMessage {
            role: User,
            content: text,
        },
    ];
    let reply = gpt_service::chat_gpt_call(params, url, context).await?;
    Ok(reply.content)
}

fn is_html_content_type(content_type: &str) -> bool {
    let mime = content_type.split(';').next().unwrap_or_default().trim();
    mime.eq_ignore_ascii_case("text/html") || mime.eq_ignore_ascii_case("application/xhtml+xml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn long_html() -> String {
        let text = "lorem ipsum dolor sit amet ".repeat(80);
        format!("<html><body><p>{text}</p></body></html>")
    }

    async fn serve(template: ResponseTemplate) -> (MockServer, String) {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/article"))
            .respond_with(template)
            .mount(&server)
            .await;
        let url = format!("{}/article", server.uri());
        (server, url)
    }

    #[test]
    fn html_content_types_are_recognised() {
        assert!(is_html_content_type("text/html"));
        assert!(is_html_content_type("text/html; charset=utf-8"));
        assert!(is_html_content_type("TEXT/HTML;charset=UTF-8"));
        assert!(is_html_content_type("application/xhtml+xml"));
        assert!(!is_html_content_type("application/pdf"));
        assert!(!is_html_content_type("text/plain"));
        assert!(!is_html_content_type(""));
    }

    #[tokio::test]
    async fn fetch_returns_text_for_html_page() {
        let (_server, url) =
            serve(ResponseTemplate::new(200).set_body_raw(long_html(), "text/html; charset=utf-8"))
                .await;

        let text = fetch_article_text(&Client::new(), &url).await.unwrap();

        assert!(text.contains("lorem ipsum"));
        assert!(!text.contains("<p>"));
    }

    #[tokio::test]
    async fn fetch_rejects_non_html_content_type() {
        let (_server, url) =
            serve(ResponseTemplate::new(200).set_body_raw(long_html(), "application/pdf")).await;

        let err = fetch_article_text(&Client::new(), &url).await.unwrap_err();

        assert!(matches!(err, ArticleSummaryError::NotHtml(ct) if ct == "application/pdf"));
    }

    #[tokio::test]
    async fn fetch_rejects_missing_content_type() {
        let (_server, url) = serve(ResponseTemplate::new(200).set_body_bytes(long_html())).await;

        let err = fetch_article_text(&Client::new(), &url).await.unwrap_err();

        assert!(matches!(err, ArticleSummaryError::NotHtml(ct) if ct.is_empty()));
    }

    #[tokio::test]
    async fn fetch_rejects_short_article() {
        let (_server, url) = serve(
            ResponseTemplate::new(200).set_body_raw("<html><body>hi</body></html>", "text/html"),
        )
        .await;

        let err = fetch_article_text(&Client::new(), &url).await.unwrap_err();

        assert!(matches!(err, ArticleSummaryError::TooShort(_)));
    }

    #[tokio::test]
    async fn fetch_rejects_error_status() {
        let (_server, url) =
            serve(ResponseTemplate::new(404).set_body_raw(long_html(), "text/html")).await;

        let err = fetch_article_text(&Client::new(), &url).await.unwrap_err();

        assert!(matches!(err, ArticleSummaryError::Fetch(_)));
    }
}
