pub fn extract_plain_text(html: &str) -> String {
    let document = scraper::Html::parse_document(html);

    document
        .root_element()
        .text()
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
