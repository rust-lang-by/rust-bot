use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

const HN_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Deserialize)]
pub struct HnItem {
    pub id: u64,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub descendants: u32,
    #[serde(default)]
    pub dead: bool,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HnStory {
    pub id: u64,
    pub title: String,
    pub url: String,
    pub score: u32,
    pub comments: u32,
}

impl HnItem {
    /// Only live stories pointing at an external article are digest material;
    /// Ask HN / text posts carry no `url`, and jobs are not `story`.
    pub fn into_linked_story(self) -> Option<HnStory> {
        if self.dead || self.deleted || self.kind.as_deref() != Some("story") {
            return None;
        }
        Some(HnStory {
            id: self.id,
            title: self.title?,
            url: self.url?,
            score: self.score,
            comments: self.descendants,
        })
    }
}

pub async fn top_story_ids(client: &Client, base_url: &str) -> Result<Vec<u64>, reqwest::Error> {
    client
        .get(format!("{base_url}/topstories.json"))
        .timeout(HN_REQUEST_TIMEOUT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

/// The API answers `null` for ids that do not exist.
pub async fn item(
    client: &Client,
    base_url: &str,
    id: u64,
) -> Result<Option<HnItem>, reqwest::Error> {
    client
        .get(format!("{base_url}/item/{id}.json"))
        .timeout(HN_REQUEST_TIMEOUT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(value: serde_json::Value) -> HnItem {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn linked_story_is_accepted() {
        let story = item(json!({
            "id": 1, "type": "story", "title": "T", "url": "https://e.com",
            "score": 10, "descendants": 4
        }))
        .into_linked_story()
        .unwrap();

        assert_eq!(
            story,
            HnStory {
                id: 1,
                title: "T".into(),
                url: "https://e.com".into(),
                score: 10,
                comments: 4
            }
        );
    }

    #[test]
    fn missing_counters_default_to_zero() {
        let story = item(json!({"id": 1, "type": "story", "title": "T", "url": "https://e.com"}))
            .into_linked_story()
            .unwrap();

        assert_eq!((story.score, story.comments), (0, 0));
    }

    #[test]
    fn non_digest_items_are_rejected() {
        let rejected = [
            json!({"id": 1, "type": "story", "title": "Ask HN: why?"}),
            json!({"id": 2, "type": "job", "title": "Hiring", "url": "https://e.com"}),
            json!({"id": 3, "type": "story", "title": "T", "url": "https://e.com", "dead": true}),
            json!({"id": 4, "type": "story", "deleted": true}),
        ];
        for value in rejected {
            assert!(item(value.clone()).into_linked_story().is_none(), "{value}");
        }
    }
}
