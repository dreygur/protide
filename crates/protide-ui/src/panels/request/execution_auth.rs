use super::*;
use super::super::request_utils::base64_encode;

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
                    auth.push(("Authorization".to_string(), format!("Bearer {}", substitute(&self.bearer_token))));
                }
            }
            AuthType::Basic => {
                if !self.basic_username.is_empty() {
                    let credentials = format!("{}:{}", substitute(&self.basic_username), substitute(&self.basic_password));
                    auth.push(("Authorization".to_string(), format!("Basic {}", base64_encode(credentials.as_bytes()))));
                }
            }
            AuthType::ApiKey => {
                if !self.api_key_name.is_empty() && self.api_key_location == ApiKeyLocation::Header {
                    auth.push((substitute(&self.api_key_name), substitute(&self.api_key_value)));
                }
            }
            AuthType::None => {}
        }
        auth
    }
}
