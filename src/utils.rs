use std::error::Error;

/// Fetches HTML content from a URL
pub async fn fetch_html(url: &str) -> Result<String, Box<dyn Error>> {
    let response = reqwest::get(url).await?;
    Ok(response.text().await?)
}

/// Checks if a string represents a disqualification status
pub fn is_dq_status(s: &str) -> bool {
    matches!(s, "DQ" | "DSQ" | "DFS" | "DNS")
}

/// Checks if a string matches a year pattern; often age for club meets and grade for collegiate
pub fn is_year_pattern(s: &str) -> bool {
    if s.len() != 2 {
        return false;
    }
    matches!(s.to_uppercase().as_str(), "FR" | "SO" | "JR" | "SR" | "GR" | "5Y" | "RS" | "FF")
        || s.chars().all(|c| c.is_ascii_digit())
}

/// Validates a string as a swim time format (e.g., 21.09, 1:08.61, 4:02.31N)
pub fn is_valid_time_format(s: &str) -> bool {
    let s = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());

    if !s.contains(':') && !s.contains('.') {
        return false;
    }

    if let Some(colon_pos) = s.find(':') {
        let before = &s[..colon_pos];
        let after = &s[colon_pos + 1..];
        return !before.is_empty()
            && before.chars().all(|c| c.is_ascii_digit())
            && after.contains('.')
            && after.len() >= 4;
    }

    if let Some(dot_pos) = s.find('.') {
        let after = &s[dot_pos + 1..];
        return after.len() >= 2 && after.chars().all(|c| c.is_ascii_digit());
    }

    false
}

/// Extracts session character (P/F) from an event URL filename
pub fn extract_session_from_url(url: &str) -> Option<char> {
    let filename = url.rsplit('/').next()?;
    let code = filename.trim_end_matches(".htm");
    let session = code.chars().rev().nth(3)?;

    match session {
        'P' | 'F' => Some(session),
        _ => None,
    }
}
