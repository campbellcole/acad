use async_trait::async_trait;
use color_eyre::{eyre::eyre, Result};
use thirtyfour::{By, WebDriver, WebElement};

#[async_trait]
trait Findable {
    async fn tfind(&self, by: impl Into<By> + Send) -> Result<WebElement>;
}

#[async_trait]
impl Findable for WebDriver {
    async fn tfind(&self, by: impl Into<By> + Send) -> Result<WebElement> {
        Ok(self.find(by).await?)
    }
}

#[async_trait]
impl Findable for WebElement {
    async fn tfind(&self, by: impl Into<By> + Send) -> Result<WebElement> {
        Ok(self.find(by).await?)
    }
}

#[async_trait]
pub trait WaitFind {
    async fn wait_find(&self, by: impl Into<By> + Send) -> Result<WebElement>;
}

#[async_trait]
impl<T: Findable + Sync> WaitFind for T {
    async fn wait_find(&self, by: impl Into<By> + Send) -> Result<WebElement> {
        let by = by.into();
        let mut attempts = 0;

        loop {
            match self.tfind(by.clone()).await {
                Ok(e) => return Ok(e),
                Err(_) => {
                    trace!("attempt {attempts} to find element: {by:?}");
                    attempts += 1;

                    if attempts > 10 {
                        return Err(eyre!("Could not find element"));
                    } else {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }
    }
}
