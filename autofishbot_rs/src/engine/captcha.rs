use crate::config::Config;
use anyhow::{Result, anyhow};
use reqwest::Client;
use log::{info, error};
use std::time::Duration;

pub struct Captcha {
    client: Client,
    config: Config,
    pub detected: bool,
    pub solving: bool,
    pub answers: Vec<String>,
    pub image_url: Option<String>,
}

impl Captcha {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::builder().timeout(Duration::from_secs(20)).build().unwrap(),
            config,
            detected: false,
            solving: false,
            answers: Vec::new(),
            image_url: None,
        }
    }

    pub async fn solve(&mut self, url: String) -> Result<String> {
        self.detected = true;
        self.solving = true;
        self.image_url = Some(url.clone());

        info!("Solving captcha: {}", url);

        // Use OCR.SPACE
        let api_key = &self.config.captcha.ocr_api_key;
        if api_key.is_empty() {
            error!("No OCR API key provided!");
            return Err(anyhow!("No OCR API key"));
        }

        let params = [
            ("apikey", api_key.as_str()),
            ("url", url.as_str()),
            ("language", "eng"),
            ("isOverlayRequired", "false"),
            ("detectOrientation", "true"),
            ("scale", "true"),
            ("OCREngine", "2"), // Engine 2 is usually better for alphanumeric
        ];

        let res = self.client.post("https://api.ocr.space/parse/image")
            .form(&params)
            .send()
            .await?;

        if !res.status().is_success() {
            self.solving = false;
             return Err(anyhow!("OCR API error: {}", res.status()));
        }

        let body: serde_json::Value = res.json().await?;

        if let Some(exit_code) = body.get("OCRExitCode").and_then(|v| v.as_i64()) {
            if exit_code != 1 {
                 self.solving = false;
                 return Err(anyhow!("OCR Error Code: {}", exit_code));
            }
        }

        let parsed_results = body.get("ParsedResults")
            .and_then(|v| v.as_array())
            .ok_or(anyhow!("No ParsedResults"))?;

        if let Some(first) = parsed_results.first() {
            let text = first.get("ParsedText").and_then(|v| v.as_str()).unwrap_or("");
            // Filter text: only alphanumeric
            let filtered: String = text.chars().filter(|c| c.is_alphanumeric()).collect();

            if filtered.len() == 6 {
                info!("Captcha solved: {}", filtered);
                self.answers.push(filtered.clone());
                self.solving = false;
                Ok(filtered)
            } else {
                self.solving = false;
                Err(anyhow!("Invalid captcha length: {} ({})", filtered.len(), filtered))
            }
        } else {
            self.solving = false;
             Err(anyhow!("No text found"))
        }
    }

    pub fn reset(&mut self) {
        self.detected = false;
        self.solving = false;
        self.answers.clear();
        self.image_url = None;
    }
}
