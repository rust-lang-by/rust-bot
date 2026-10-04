use super::hn_api::HnStory;

#[derive(Debug, Clone, PartialEq)]
pub struct DigestEntry {
    pub story: HnStory,
    pub summary: String,
}

/// One Telegram message (HTML parse mode) per digest entry.
pub fn format_entry(rank: usize, entry: &DigestEntry) -> String {
    let story = &entry.story;
    format!(
        "#{rank} <a href=\"{url}\">{title}</a>\n▲ {score} · 💬 <a href=\"{hn_url}\">{comments} comments</a>\n\nTLDR:\n{summary}",
        url = escape_html(&story.url),
        title = escape_html(&story.title),
        score = story.score,
        hn_url = hn_discussion_url(story.id),
        comments = story.comments,
        summary = escape_html(&entry.summary),
    )
}

fn hn_discussion_url(id: u64) -> String {
    format!("https://news.ycombinator.com/item?id={id}")
}

// teloxide's `utils::html::escape` leaves `"` alone, which breaks `href="..."`.
fn escape_html(text: &str) -> String {
    text.chars()
        .fold(String::with_capacity(text.len()), |mut out, c| {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                c => out.push(c),
            }
            out
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str, url: &str, summary: &str) -> DigestEntry {
        DigestEntry {
            story: HnStory {
                id: 42,
                title: title.into(),
                url: url.into(),
                score: 456,
                comments: 123,
            },
            summary: summary.into(),
        }
    }

    #[test]
    fn formats_entry_with_links_and_counters() {
        let text = format_entry(1, &entry("Title", "https://e.com/a", "- point"));

        assert_eq!(
            text,
            "#1 <a href=\"https://e.com/a\">Title</a>\n\
             ▲ 456 · 💬 <a href=\"https://news.ycombinator.com/item?id=42\">123 comments</a>\n\n\
             TLDR:\n- point"
        );
    }

    #[test]
    fn escapes_user_controlled_text() {
        let text = format_entry(
            2,
            &entry(
                "Rust <3 & \"you\"",
                "https://e.com/?a=1&b=\"2\"",
                "use <T> & go",
            ),
        );

        assert!(text.contains("Rust &lt;3 &amp; &quot;you&quot;</a>"));
        assert!(text.contains("href=\"https://e.com/?a=1&amp;b=&quot;2&quot;\""));
        assert!(text.ends_with("use &lt;T&gt; &amp; go"));
    }
}
