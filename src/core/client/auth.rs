//! Cookie & crumb acquisition for Yahoo endpoints.

use crate::core::error::YfError;
use reqwest::header::SET_COOKIE;

impl super::YfClient {
    pub(crate) async fn ensure_credentials(&self) -> Result<(), YfError> {
        // First check with a read lock is fast and allows concurrency.
        if self.state.read().await.crumb.is_some() {
            return Ok(());
        }

        // If the crumb is missing, acquire a write lock.
        let state = self.state.write().await;

        // Double-check: another task might have acquired the lock and set credentials
        // while this one was waiting.
        if state.crumb.is_some() {
            return Ok(());
        }

        // Now, with the write lock held, fetch the credentials.
        // We need to temporarily release the state lock to make HTTP calls.
        drop(state);
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
            .ok_or(YfError::Data("No cookie received from fc.yahoo.com".into()))?
            .to_str()
            .map_err(|_| YfError::Data("Invalid cookie header format".into()))?
            .to_string();

        let mut state = self.state.write().await;
        state.cookie = Some(cookie);
        Ok(())
    }

    async fn get_crumb_internal(&self) -> Result<(), YfError> {
        let state = self.state.read().await;
        if state.cookie.is_none() {
            return Err(YfError::Data("Cookie is missing, cannot get crumb".into()));
        }
        drop(state); // release read lock before making http call

        let url = self.crumb_url.clone();
        let req = self.http.get(url);
        let resp = self.send_with_retry(req, None).await?;
        let crumb = resp.text().await?;

        if crumb.is_empty() || crumb.contains('{') || crumb.contains('<') {
            return Err(YfError::Data(format!("Received invalid crumb: {}", crumb)));
        }

        let mut state = self.state.write().await;
        state.crumb = Some(crumb);
        Ok(())
    }
}
