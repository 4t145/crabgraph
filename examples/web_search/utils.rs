use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationSegment {
    pub label: String,
    pub short_url: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub start_index: i32,
    pub end_index: i32,
    pub segments: Vec<CitationSegment>,
}

impl Message {
    pub fn get_research_topic(list: &[Self]) -> String {
        if list.len() == 1 {
            return list[0].content().to_owned();
        } else {
            return list.iter().fold(String::new(), |mut s, m| {
                match m {
                    Message::Ai(ai_message) => {
                        s.push_str(&format!("Assistant: {}\n", ai_message.content));
                    }
                    Message::Human(human_message) => {
                        s.push_str(&format!("User: {}\n", human_message.content));
                    }
                }
                s
            });
        }
    }
}

pub fn schema<T: schemars::JsonSchema>() -> serde_json::Value {
    let mut settings = schemars::generate::SchemaSettings::draft07();
    settings.meta_schema = None;
    settings.inline_subschemas = true;

    let generator = settings.into_generator();
    let root_schema = generator.into_root_schema_for::<T>();
    root_schema.into()
}

pub fn resolve_urls(urls_to_resolve: Value, id: usize) -> HashMap<String, String> {
    const PREFIX: &str = "https://vertexaisearch.cloud.google.com/id/";
    urls_to_resolve
        .as_array()
        .unwrap_or(&vec![])
        .into_iter()
        .filter_map(|site| site["web"]["uri"].as_str())
        .enumerate()
        .fold(HashMap::default(), |mut map, (index, uri)| {
            if !map.contains_key(uri) {
                map.insert(uri.to_owned(), format!("{PREFIX}{id}-{index}"));
            }
            map
        })
}

/// Extracts and formats citation information from a Gemini model's response.
///
/// This function processes the grounding metadata provided in the response to
/// construct a list of citation objects. Each citation object includes the
/// start and end indices of the text segment it refers to, and a string
/// containing formatted markdown links to the supporting web chunks.
///
/// # Arguments
///
/// * `response` - The response object from the Gemini model as a serde_json::Value,
///                expected to have a structure including `candidates[0].grounding_metadata`.
/// * `resolved_urls_map` - A HashMap mapping chunk URIs to resolved URLs.
///
/// # Returns
///
/// A Vec of Citation structs, where each Citation represents a citation
/// and has the following fields:
/// - `start_index` (i32): The starting character index of the cited
///                        segment in the original text. Defaults to 0
///                        if not specified.
/// - `end_index` (i32): The character index immediately after the
///                      end of the cited segment (exclusive).
/// - `segments` (Vec<CitationSegment>): A list of individual citation segments
///                                      for each grounding chunk.
/// Returns an empty vector if no valid candidates or grounding supports
/// are found, or if essential data is missing.
pub fn get_citations(
    response: &Value,
    resolved_urls_map: &HashMap<String, String>,
) -> Vec<Citation> {
    let mut citations = Vec::new();

    // Ensure response and necessary nested structures are present
    let candidates = match response.get("candidates").and_then(|c| c.as_array()) {
        Some(candidates) if !candidates.is_empty() => candidates,
        _ => return citations,
    };

    let candidate = &candidates[0];
    let grounding_metadata = match candidate.get("groundingMetadata") {
        Some(metadata) => metadata,
        None => return citations,
    };

    let grounding_supports = match grounding_metadata
        .get("groundingSupports")
        .and_then(|s| s.as_array())
    {
        Some(supports) => supports,
        None => return citations,
    };

    let grounding_chunks = match grounding_metadata
        .get("groundingChunks")
        .and_then(|c| c.as_array())
    {
        Some(chunks) => chunks,
        None => return citations,
    };

    for support in grounding_supports {
        // Ensure segment information is present
        let segment = match support.get("segment") {
            Some(segment) => segment,
            None => continue, // Skip this support if segment info is missing
        };

        let start_index = segment
            .get("startIndex")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        // Ensure end_index is present to form a valid segment
        let end_index = match segment.get("endIndex").and_then(|v| v.as_i64()) {
            Some(end_index) => end_index as i32,
            None => continue, // Skip if end_index is missing, as it's crucial
        };

        let mut citation_segments = Vec::new();

        if let Some(grounding_chunk_indices) = support
            .get("groundingChunkIndices")
            .and_then(|i| i.as_array())
        {
            for index_value in grounding_chunk_indices {
                if let Some(index) = index_value.as_u64() {
                    let index = index as usize;
                    if let Some(chunk) = grounding_chunks.get(index) {
                        if let (Some(uri), Some(title)) = (
                            chunk
                                .get("web")
                                .and_then(|w| w.get("uri"))
                                .and_then(|u| u.as_str()),
                            chunk
                                .get("web")
                                .and_then(|w| w.get("title"))
                                .and_then(|t| t.as_str()),
                        ) {
                            let resolved_url = resolved_urls_map
                                .get(uri)
                                .cloned()
                                .unwrap_or_else(|| uri.to_string());

                            // Extract label from title (remove extension if present)
                            let label = if let Some(dot_index) = title.rfind('.') {
                                title[..dot_index].to_string()
                            } else {
                                title.to_string()
                            };

                            citation_segments.push(CitationSegment {
                                label,
                                short_url: resolved_url,
                                value: uri.to_string(),
                            });
                        }
                    }
                }
            }
        }

        citations.push(Citation {
            start_index,
            end_index,
            segments: citation_segments,
        });
    }

    citations
}

/// Inserts citation markers into a text string based on start and end indices.
///
/// # Arguments
///
/// * `text` - The original text string.
/// * `citations_list` - A slice of Citation structs, where each Citation
///                      contains start_index, end_index, and segments
///                      (the markers to insert). Indices are assumed to be
///                      for the original text.
///
/// # Returns
///
/// The text with citation markers inserted as a String.
pub fn insert_citation_markers(text: &str, citations_list: &[Citation]) -> String {
    // Sort citations by end_index in descending order.
    // If end_index is the same, secondary sort by start_index descending.
    // This ensures that insertions at the end of the string don't affect
    // the indices of earlier parts of the string that still need to be processed.
    let mut sorted_citations = citations_list.to_vec();
    sorted_citations.sort_by(|a, b| {
        match b.end_index.cmp(&a.end_index) {
            std::cmp::Ordering::Equal => b.start_index.cmp(&a.start_index),
            other => other,
        }
    });

    let mut modified_text = text.to_string();
    
    for citation_info in sorted_citations {
        // These indices refer to positions in the *original* text,
        // but since we iterate from the end, they remain valid for insertion
        // relative to the parts of the string already processed.
        let end_idx = citation_info.end_index as usize;
        
        let mut marker_to_insert = String::new();
        for segment in &citation_info.segments {
            marker_to_insert.push_str(&format!(" [{}]({})", segment.label, segment.short_url));
        }
        
        // Insert the citation marker at the original end_idx position
        // Ensure we don't go out of bounds
        if end_idx <= modified_text.len() {
            modified_text.insert_str(end_idx, &marker_to_insert);
        }
    }

    modified_text
}
