
use super::*;
use crate::components::{is_word_char, find_word_start, find_word_end};
use crate::panels::request_utils::{url_encode, url_decode, base64_encode};

// ===== Unit Tests for HTTP Methods =====

#[test]
fn test_http_method_as_str() {
assert_eq!(HttpMethod::Get.as_str(), "GET");
assert_eq!(HttpMethod::Post.as_str(), "POST");
assert_eq!(HttpMethod::Put.as_str(), "PUT");
assert_eq!(HttpMethod::Patch.as_str(), "PATCH");
assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
}

#[test]
fn test_http_method_from_str() {
assert_eq!(HttpMethod::from_str("GET"), Some(HttpMethod::Get));
assert_eq!(HttpMethod::from_str("get"), Some(HttpMethod::Get));
assert_eq!(HttpMethod::from_str("POST"), Some(HttpMethod::Post));
assert_eq!(HttpMethod::from_str("PUT"), Some(HttpMethod::Put));
assert_eq!(HttpMethod::from_str("PATCH"), Some(HttpMethod::Patch));
assert_eq!(HttpMethod::from_str("DELETE"), Some(HttpMethod::Delete));
assert_eq!(HttpMethod::from_str("INVALID"), Some(HttpMethod::Custom("INVALID".to_string())));
assert_eq!(HttpMethod::from_str(""), None);
}

#[test]
fn test_http_method_all() {
let methods = HttpMethod::all();
assert_eq!(methods.len(), 5);
assert!(methods.contains(&HttpMethod::Get));
assert!(methods.contains(&HttpMethod::Post));
assert!(methods.contains(&HttpMethod::Put));
assert!(methods.contains(&HttpMethod::Patch));
assert!(methods.contains(&HttpMethod::Delete));
}

// ===== Unit Tests for Word Boundary Functions =====

#[test]
fn test_is_word_char() {
assert!(is_word_char('a'));
assert!(is_word_char('Z'));
assert!(is_word_char('5'));
assert!(is_word_char('_'));
assert!(!is_word_char(' '));
assert!(!is_word_char('.'));
assert!(!is_word_char('/'));
assert!(!is_word_char(':'));
}

#[test]
fn test_find_word_start_simple() {
let text = "hello world";
assert_eq!(find_word_start(text, 0), 0);
assert_eq!(find_word_start(text, 3), 0);
assert_eq!(find_word_start(text, 5), 0); // end of "hello"
assert_eq!(find_word_start(text, 6), 6); // space -> finds "world"
assert_eq!(find_word_start(text, 8), 6); // middle of "world"
}

#[test]
fn test_find_word_end_simple() {
let text = "hello world";
assert_eq!(find_word_end(text, 0), 5);
assert_eq!(find_word_end(text, 3), 5);
assert_eq!(find_word_end(text, 5), 11); // at space, skips to next word end
assert_eq!(find_word_end(text, 6), 11);
assert_eq!(find_word_end(text, 8), 11);
}

#[test]
fn test_find_word_boundaries_url() {
let text = "https://api.example.com/users";
// "https" is a word
assert_eq!(find_word_start(text, 2), 0);
assert_eq!(find_word_end(text, 2), 5);
// "api" is a word
assert_eq!(find_word_start(text, 9), 8);
assert_eq!(find_word_end(text, 9), 11);
// "users" is a word
assert_eq!(find_word_start(text, 27), 24);
assert_eq!(find_word_end(text, 27), 29);
}

#[test]
fn test_find_word_boundaries_empty() {
assert_eq!(find_word_start("", 0), 0);
assert_eq!(find_word_end("", 0), 0);
}

#[test]
fn test_find_word_boundaries_single_word() {
let text = "hello";
assert_eq!(find_word_start(text, 0), 0);
assert_eq!(find_word_start(text, 2), 0);
assert_eq!(find_word_start(text, 5), 0);
assert_eq!(find_word_end(text, 0), 5);
assert_eq!(find_word_end(text, 2), 5);
assert_eq!(find_word_end(text, 5), 5);
}

#[test]
fn test_find_word_with_underscore() {
let text = "hello_world test";
// "hello_world" is treated as one word (underscore is word char)
assert_eq!(find_word_start(text, 5), 0);
assert_eq!(find_word_end(text, 5), 11);
}

// ===== Unit Tests for URL Encoding/Decoding =====
// Note: Uses application/x-www-form-urlencoded style (+ for spaces)

#[test]
fn test_url_encode() {
assert_eq!(url_encode("hello"), "hello");
assert_eq!(url_encode("hello world"), "hello+world"); // + for spaces
assert_eq!(url_encode("key=value"), "key%3Dvalue");
assert_eq!(url_encode("a&b"), "a%26b");
assert_eq!(url_encode("100%"), "100%25");
}

#[test]
fn test_url_decode() {
assert_eq!(url_decode("hello"), "hello");
assert_eq!(url_decode("hello+world"), "hello world"); // + decoded to space
assert_eq!(url_decode("hello%20world"), "hello world"); // %20 also works
assert_eq!(url_decode("key%3Dvalue"), "key=value");
assert_eq!(url_decode("a%26b"), "a&b");
assert_eq!(url_decode("100%25"), "100%");
}

#[test]
fn test_url_encode_decode_roundtrip() {
let test_cases = vec![
"simple",
"with spaces",
"special=chars&here",
"numbers123",
];
for original in test_cases {
let encoded = url_encode(original);
let decoded = url_decode(&encoded);
assert_eq!(decoded, original, "Roundtrip failed for: {}", original);
}
}

#[test]
fn test_url_encode_special_chars() {
assert_eq!(url_encode("?"), "%3F");
assert_eq!(url_encode("#"), "%23");
assert_eq!(url_encode("/"), "%2F");
assert_eq!(url_encode(":"), "%3A");
assert_eq!(url_encode("+"), "%2B");
}

#[test]
fn test_url_decode_invalid() {
// Incomplete percent encoding should be handled gracefully
assert_eq!(url_decode("%"), "%");
assert_eq!(url_decode("%2"), "%2");
assert_eq!(url_decode("%GG"), "%GG"); // Invalid hex
}

// ===== Unit Tests for Base64 Encoding =====

#[test]
fn test_base64_encode() {
assert_eq!(base64_encode(b""), "");
assert_eq!(base64_encode(b"f"), "Zg==");
assert_eq!(base64_encode(b"fo"), "Zm8=");
assert_eq!(base64_encode(b"foo"), "Zm9v");
assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
}

#[test]
fn test_base64_encode_basic_auth() {
// Test basic auth style encoding (username:password)
assert_eq!(base64_encode(b"user:pass"), "dXNlcjpwYXNz");
assert_eq!(base64_encode(b"admin:secret123"), "YWRtaW46c2VjcmV0MTIz");
}

// ===== Unit Tests for Data Types =====

#[test]
fn test_key_value_pair_default() {
let pair = KeyValuePair::default();
assert_eq!(pair.key, "");
assert_eq!(pair.value, "");
assert!(!pair.enabled);
}

#[test]
fn test_key_value_pair_creation() {
let pair = KeyValuePair {
key: "Content-Type".to_string(),
value: "application/json".to_string(),
enabled: true,
};
assert_eq!(pair.key, "Content-Type");
assert_eq!(pair.value, "application/json");
assert!(pair.enabled);
}

#[test]
fn test_auth_type_default() {
let auth = AuthType::default();
assert_eq!(auth, AuthType::None);
}

#[test]
fn test_auth_type_variants() {
assert_ne!(AuthType::None, AuthType::Bearer);
assert_ne!(AuthType::Bearer, AuthType::Basic);
assert_ne!(AuthType::Basic, AuthType::ApiKey);
}

#[test]
fn test_api_key_location_default() {
let location = ApiKeyLocation::default();
assert_eq!(location, ApiKeyLocation::Header);
}

#[test]
fn test_api_key_location_variants() {
assert_ne!(ApiKeyLocation::Header, ApiKeyLocation::QueryParam);
}

// ===== Unit Tests for Edit Target =====

#[test]
fn test_edit_target_equality() {
assert_eq!(EditTarget::Body, EditTarget::Body);
assert_eq!(EditTarget::HeaderKey(0), EditTarget::HeaderKey(0));
assert_ne!(EditTarget::HeaderKey(0), EditTarget::HeaderKey(1));
assert_ne!(EditTarget::HeaderKey(0), EditTarget::HeaderValue(0));
}

#[test]
fn test_edit_target_param_indices() {
let target1 = EditTarget::ParamKey(5);
let target2 = EditTarget::ParamValue(5);
assert_ne!(target1, target2);

if let EditTarget::ParamKey(idx) = target1 {
assert_eq!(idx, 5);
} else {
panic!("Expected ParamKey");
}
}

// ===== Integration-like Tests (testing logic without GPUI) =====

/// Test URL query string parsing logic
#[test]
fn test_parse_query_string() {
// Simulate the logic from sync_params_from_url
let url = "https://api.example.com/users?name=john&age=30&active=true";

let query_start = url.find('?').unwrap();
let query_string = &url[query_start + 1..];

let params: Vec<KeyValuePair> = query_string
.split('&')
.filter(|pair| !pair.is_empty())
.map(|pair| {
let mut parts = pair.splitn(2, '=');
let key = url_decode(parts.next().unwrap_or(""));
let value = url_decode(parts.next().unwrap_or(""));
KeyValuePair {
    key,
    value,
    enabled: true,
}
})
.collect();

assert_eq!(params.len(), 3);
assert_eq!(params[0].key, "name");
assert_eq!(params[0].value, "john");
assert_eq!(params[1].key, "age");
assert_eq!(params[1].value, "30");
assert_eq!(params[2].key, "active");
assert_eq!(params[2].value, "true");
}

/// Test URL building from params logic
#[test]
fn test_build_query_string() {
// Simulate the logic from sync_url_from_params
let base_url = "https://api.example.com/search";
let params = vec![
KeyValuePair {
key: "q".to_string(),
value: "rust programming".to_string(),
enabled: true,
},
KeyValuePair {
key: "limit".to_string(),
value: "10".to_string(),
enabled: true,
},
KeyValuePair {
key: "debug".to_string(),
value: "true".to_string(),
enabled: false, // Disabled, should not appear in URL
},
];

let query_parts: Vec<String> = params
.iter()
.filter(|p| p.enabled && !p.key.is_empty())
.map(|p| {
if p.value.is_empty() {
    url_encode(&p.key)
} else {
    format!("{}={}", url_encode(&p.key), url_encode(&p.value))
}
})
.collect();

let url = if query_parts.is_empty() {
base_url.to_string()
} else {
format!("{}?{}", base_url, query_parts.join("&"))
};

assert!(url.contains("q=rust+programming")); // + for spaces
assert!(url.contains("limit=10"));
assert!(!url.contains("debug")); // Disabled param excluded
}

/// Test empty query handling
#[test]
fn test_empty_query_string() {
let url = "https://api.example.com/users";
assert!(url.find('?').is_none());
}

/// Test key-only params (no value)
#[test]
fn test_key_only_param() {
let param = KeyValuePair {
key: "verbose".to_string(),
value: "".to_string(),
enabled: true,
};

let encoded = if param.value.is_empty() {
url_encode(&param.key)
} else {
format!("{}={}", url_encode(&param.key), url_encode(&param.value))
};

assert_eq!(encoded, "verbose");
}
