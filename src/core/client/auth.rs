//! Cookie & crumb acquisition for Yahoo endpoints.

use crate::core::error::YfError;
use reqwest::header::SET_COOKIE;

impl super::YfClient {
    /// Ensure we have a valid cookie + crumb pair.
    pub(crate) async fn ensure_credentials(&mut self) -> Result<(), YfError> {
        if self.crumb.is_some() {
            return Ok(());
        }
        self.get_cookie().await?;
        self.get_crumb_internal().await?;
        Ok(())
    }

    /// Clear the crumb (used when API signals an invalid crumb and we need to retry).
    pub(crate) fn clear_crumb(&mut self) {
        self.crumb = None;
    }

    /// Get the crumb string (if any).
    pub(crate) fn crumb(&self) -> Option<&str> {
        self.crumb.as_deref()
    }

    async fn get_cookie(&mut self) -> Result<(), YfError> {
        let req = self.http.get(self.cookie_url.clone());
        let resp = self.send_with_retry(req, None).await?;

        let cookie = resp
            .headers()
            .get(SET_COOKIE)
            .ok_or(YfError::Data("No cookie received from fc.yahoo.com".into()))?
            .to_str()
            .map_err(|_| YfError::Data("Invalid cookie header format".into()))?
            .to_string();

        self.cookie = Some(cookie);
        Ok(())
    }

    async fn get_crumb_internal(&mut self) -> Result<(), YfError> {
        if self.cookie.is_none() {
            return Err(YfError::Data("Cookie is missing, cannot get crumb".into()));
        }
        let url = self.crumb_url.clone();
        let req = self.http.get(url);
        let resp = self.send_with_retry(req, None).await?;
        let crumb = resp.text().await?;

        if crumb.is_empty() || crumb.contains('{') || crumb.contains('<') {
            return Err(YfError::Data(format!("Received invalid crumb: {}", crumb)));
        }

        self.crumb = Some(crumb);
        Ok(())
    }
}
