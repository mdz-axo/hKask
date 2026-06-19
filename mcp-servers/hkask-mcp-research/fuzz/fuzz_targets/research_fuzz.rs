//! Research MCP server fuzz targets.
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types
//!
//! Request types live in two modules:
//!   - hkask_mcp_research::types (web search/extract/browse/find_similar)
//!   - hkask_mcp_research::rss_types (RSS subscribe/fetch/mark/unread count/import)

use bolero::check;
use hkask_mcp_research::rss_types;
use hkask_mcp_research::types;

#[test]
fn fuzz_research_deserialize_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        // Web search / extract / browse types (types module)
        let _ = serde_json::from_str::<types::SearchRequest>(s);
        let _ = serde_json::from_str::<types::FindSimilarRequest>(s);
        let _ = serde_json::from_str::<types::ExtractRequest>(s);
        let _ = serde_json::from_str::<types::BrowseRequest>(s);

        // RSS types (rss_types module)
        let _ = serde_json::from_str::<rss_types::SubscribeRequest>(s);
        let _ = serde_json::from_str::<rss_types::UnsubscribeRequest>(s);
        let _ = serde_json::from_str::<rss_types::ListSubscriptionsRequest>(s);
        let _ = serde_json::from_str::<rss_types::FetchRequest>(s);
        let _ = serde_json::from_str::<rss_types::GetEntriesRequest>(s);
        let _ = serde_json::from_str::<rss_types::MarkReadRequest>(s);
        let _ = serde_json::from_str::<rss_types::UnreadCountRequest>(s);
        let _ = serde_json::from_str::<rss_types::ImportOpmlRequest>(s);
        let _ = serde_json::from_str::<rss_types::DiscoverRequest>(s);
        let _ = serde_json::from_str::<rss_types::EditTagRequest>(s);
    });
}
