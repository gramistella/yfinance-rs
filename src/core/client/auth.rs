//! Cookie & crumb acquisition for Yahoo endpoints.

use crate::core::error::YfError;
use reqwest::header::SET_COOKIE;

impl super::YfClient {
    pub(crate) async fn ensure_credentials(&self) -> Result<(), YfError> {
        // Fast path: check if credentials exist with a read lock.
        if self.state.read().await.crumb.is_some() {
            return Ok(());
        }

        // Slow path: acquire the dedicated fetch lock to ensure only one task proceeds.
        let _guard = self.credential_fetch_lock.lock().await;

        // Double-check: another task might have fetched credentials while this one was waiting.
        if self.state.read().await.crumb.is_some() {
            return Ok(());
        }

        // With the lock held, we can safely perform the network operations.
        self.get_cookie().await?;
        self.get_crumb_internal().await?;

        Ok(())
    }

    pub(crate) async fn clear_crumb(&self) {
        let mut state = self.state.write().await;
        state.crumb = None;
    }

    pub(crate) async fn crumb(&self) -> Option<String> {
        let state = self.state.read().await;
        state.crumb.clone()
    }

    async fn get_cookie(&self) -> Result<(), YfError> {
        let req = self.http.get(self.cookie_url.clone());
        let resp = self.send_with_retry(req, None).await?;

        let cookie = resp
            .headers()
            .get(SET_COOKIE)
            .ok_or(YfError::Auth("No cookie received from fc.yahoo.com".into()))?
            .to_str()
            .map_err(|_| YfError::Auth("Invalid cookie header format".into()))?
            .to_string();

        self.state.write().await.cookie = Some(cookie);
        Ok(())
    }

    async fn get_crumb_internal(&self) -> Result<(), YfError> {
        let state = self.state.read().await;
        if state.cookie.is_none() {
            return Err(YfError::Auth("Cookie is missing, cannot get crumb".into()));
        }
        drop(state); // release read lock before making http call

        let url = self.crumb_url.clone();
        let req = self.http.get(url);
        let resp = self.send_with_retry(req, None).await?;
        let crumb = resp.text().await?;

        if crumb.is_empty() || crumb.contains('{') || crumb.contains('<') {
            return Err(YfError::Auth(format!("Received invalid crumb: {crumb}")));
        }

        self.state.write().await.crumb = Some(crumb);
        Ok(())
    }
}
