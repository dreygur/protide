use super::super::request_utils::{base64_encode, url_encode};
use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Build auth headers from the current auth configuration.
    /// `substitute` is applied to all field values before use.
    pub(super) fn build_auth_headers<F>(&self, substitute: &F) -> Vec<(String, String)>
    where
        F: Fn(&str) -> String,
    {
        let mut auth = Vec::new();
        match self.auth_type {
            AuthType::Bearer => {
                if !self.bearer_token.is_empty() {
                    auth.push((
                        "Authorization".to_string(),
                        format!("Bearer {}", substitute(&self.bearer_token)),
                    ));
                }
            }
            AuthType::Basic => {
                if !self.basic_username.is_empty() {
                    let credentials = format!(
                        "{}:{}",
                        substitute(&self.basic_username),
                        substitute(&self.basic_password)
                    );
                    auth.push((
                        "Authorization".to_string(),
                        format!("Basic {}", base64_encode(credentials.as_bytes())),
                    ));
                }
            }
            AuthType::ApiKey => {
                if !self.api_key_name.is_empty() && self.api_key_location == ApiKeyLocation::Header
                {
                    auth.push((
                        substitute(&self.api_key_name),
                        substitute(&self.api_key_value),
                    ));
                }
            }
            AuthType::None => {}
        }
        auth
    }

    /// Append the API key to `url` when auth sends it as a query parameter.
    pub(super) fn with_auth_query<F>(&self, url: String, substitute: &F) -> String
    where
        F: Fn(&str) -> String,
    {
        let name = substitute(&self.api_key_name);
        let value = substitute(&self.api_key_value);
        if self.auth_type != AuthType::ApiKey
            || self.api_key_location != ApiKeyLocation::QueryParam
            || name.is_empty()
            || value.is_empty()
        {
            return url;
        }
        let separator = if url.contains('?') { '&' } else { '?' };
        format!(
            "{url}{separator}{}={}",
            url_encode(&name),
            url_encode(&value)
        )
    }
}
