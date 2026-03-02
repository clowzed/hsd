use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// CRPT API client for verifying Honest Sign codes.
#[derive(Debug, Clone)]
pub struct CrptClient {
    client: Client,
    base_url: String,
}

/// Errors that can occur when calling the CRPT API.
#[derive(Debug, Error)]
pub enum CrptError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API returned error status: {0}")]
    ApiError(String),

    #[error("Failed to parse API response: {0}")]
    ParseError(String),
}

/// Request body for the CRPT check endpoint.
#[derive(Debug, Serialize)]
struct CheckRequest {
    code: String,
}

/// Response from the CRPT check endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrptResponse {
    /// Request ID
    pub id: Option<i64>,

    /// Whether the code was found in the system
    pub code_founded: bool,

    /// Code status (ok, wrong, etc.)
    pub status: Option<String>,

    /// Code status v2
    pub status_v2: Option<String>,

    /// Whether the code is verified
    pub verified: Option<bool>,

    /// Whether the code is known
    pub known: Option<bool>,

    /// Product category
    pub category: Option<String>,

    /// The code that was checked
    pub code: Option<String>,

    /// GTIN (14-digit product code)
    pub gtin: Option<String>,

    /// Serial number
    pub serial: Option<String>,

    /// Product name
    pub product_name: Option<String>,

    /// Outer status (IN_CIRCULATION, RETIRED, WITHDRAWN, APPLIED, etc.)
    pub outer_status: Option<String>,

    /// Emission type (LOCAL, etc.)
    pub emission_type: Option<String>,

    /// Pack type (UNIT, etc.)
    pub pack_type: Option<String>,

    /// Withdraw reason if RETIRED
    pub withdraw_reason: Option<String>,

    /// Production date (timestamp)
    pub produced_date: Option<i64>,

    /// Introduction date (timestamp)
    pub introduced_date: Option<i64>,

    /// Expiration date (timestamp)
    pub expire_date: Option<i64>,

    /// Whether the code is blocked
    pub is_blocked: Option<bool>,

    /// Screen data with product info
    pub screen: Option<ScreenData>,
}

/// Screen data containing product details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenData {
    pub items: Option<Vec<ScreenItem>>,
}

/// Individual screen item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenItem {
    pub order: Option<i32>,
    pub item_type: Option<String>,
    pub title: Option<String>,
    pub status_card: Option<StatusCard>,
    pub attr_list: Option<Vec<Attribute>>,
}

/// Status card with status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusCard {
    pub status_type: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

/// Product attribute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub label: Option<String>,
    pub value: Option<String>,
}

/// Color hint for status display in UI.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[allow(dead_code)]
pub enum StatusColorHint {
    /// Green - good status (IN_CIRCULATION)
    Green,
    /// Yellow - pending/waiting status
    Yellow,
    /// Red - bad status (RETIRED, WITHDRAWN)
    Red,
    /// Gray - unknown status
    Gray,
}

impl CrptClient {
    /// Creates a new CRPT API client.
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: CRPT_API_BASE_URL.to_string(),
        }
    }

    /// Checks a Honest Sign code against the CRPT API.
    ///
    /// # Arguments
    /// * `code` - The raw Honest Sign code (as received from scanner)
    ///
    /// # Returns
    /// The API response containing product information and status.
    pub async fn check_code(&self, code: &str) -> Result<CrptResponse, CrptError> {
        let url = format!("{}/v2/mobile/check", self.base_url);

        let request = CheckRequest {
            code: code.to_string(),
        };

        tracing::debug!("Checking code with CRPT API: {}", code);

        let response = self
            .client
            .post(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Connection", "keep-alive")
            .header("Pragma", "no-cache")
            .header("Cache-Control", "no-cache")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CrptError::ApiError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let crpt_response: CrptResponse = response.json().await.map_err(|e| {
            CrptError::ParseError(format!("Failed to parse response: {}", e))
        })?;

        tracing::debug!(
            "CRPT response: code_founded={}, outer_status={:?}",
            crpt_response.code_founded,
            crpt_response.outer_status
        );

        Ok(crpt_response)
    }
}

impl Default for CrptClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CrptResponse {
    /// Returns true if the code is in circulation and can be used.
    #[allow(dead_code)]
    pub fn is_in_circulation(&self) -> bool {
        self.outer_status.as_deref() == Some("IN_CIRCULATION")
    }

    /// Returns whether this is a valid/acceptable status for adding to buffer.
    /// Accepts IN_CIRCULATION and INTRODUCED statuses for printing labels.
    pub fn is_acceptable_for_label(&self) -> bool {
        matches!(
            self.outer_status.as_deref(),
            Some("IN_CIRCULATION") | Some("INTRODUCED")
        )
    }

    /// Returns the Russian status message.
    pub fn status_message_ru(&self) -> String {
        match self.outer_status.as_deref() {
            Some("IN_CIRCULATION") => "В обороте".to_string(),
            Some("RETIRED") => "Выбыл из оборота".to_string(),
            Some("WITHDRAWN") => "Выведен из оборота".to_string(),
            Some("APPLIED") => "На регистрации".to_string(),
            Some("WAIT_SHIPMENT") => "Ожидает отгрузки".to_string(),
            Some("WAIT_ACCEPTANCE") => "Ожидает приёмки".to_string(),
            Some("WAIT_TRANSFER_TO_OWNER") => "Ожидает передачи".to_string(),
            Some("INTRODUCED") => "Введён в оборот".to_string(),
            Some("EMITTED") => "Эмитирован".to_string(),
            Some(status) => format!("Статус: {}", status),
            None => "Статус не определён".to_string(),
        }
    }

    /// Returns the Russian status explanation.
    pub fn status_explanation_ru(&self) -> String {
        match self.outer_status.as_deref() {
            Some("IN_CIRCULATION") => "Товар легально в продаже".to_string(),
            Some("RETIRED") => "Товар уже был продан или списан".to_string(),
            Some("WITHDRAWN") => "Товар снят с продажи производителем".to_string(),
            Some("APPLIED") => "Код ещё не введён в оборот".to_string(),
            Some("WAIT_SHIPMENT") => "Товар ожидает отгрузки со склада".to_string(),
            Some("WAIT_ACCEPTANCE") => "Товар ожидает приёмки получателем".to_string(),
            Some("WAIT_TRANSFER_TO_OWNER") => "Товар ожидает передачи владельцу".to_string(),
            Some("INTRODUCED") => "Товар введён в оборот, но ещё не в продаже".to_string(),
            Some("EMITTED") => "Код эмитирован, но ещё не введён в оборот".to_string(),
            Some(status) => format!("Неизвестный статус API: {}. Обратитесь в поддержку.", status),
            None => {
                // Try to get more info from other fields
                if let Some(status) = &self.status {
                    format!("Статус проверки: {}", status)
                } else if let Some(status_v2) = &self.status_v2 {
                    format!("Статус v2: {}", status_v2)
                } else {
                    "API не вернул статус товара".to_string()
                }
            }
        }
    }

    /// Returns a color hint for UI display
    #[allow(dead_code)]
    pub fn status_color_hint(&self) -> StatusColorHint {
        match self.outer_status.as_deref() {
            Some("IN_CIRCULATION") => StatusColorHint::Green,
            Some("RETIRED") | Some("WITHDRAWN") => StatusColorHint::Red,
            Some("APPLIED") | Some("EMITTED") => StatusColorHint::Yellow,
            Some("WAIT_SHIPMENT") | Some("WAIT_ACCEPTANCE") | Some("WAIT_TRANSFER_TO_OWNER") | Some("INTRODUCED") => StatusColorHint::Yellow,
            _ => StatusColorHint::Gray,
        }
    }

    /// Returns the formatted production date.
    pub fn formatted_produced_date(&self) -> Option<String> {
        self.produced_date.map(|ts| {
            let datetime = chrono::DateTime::from_timestamp(ts / 1000, 0)
                .unwrap_or_default();
            datetime.format("%d.%m.%Y").to_string()
        })
    }

    /// Returns the formatted expiration date.
    pub fn formatted_expire_date(&self) -> Option<String> {
        self.expire_date.map(|ts| {
            let datetime = chrono::DateTime::from_timestamp(ts / 1000, 0)
                .unwrap_or_default();
            datetime.format("%d.%m.%Y").to_string()
        })
    }

    /// Extracts vendor code (артикул) from screen attributes.
    pub fn vendor_code(&self) -> Option<String> {
        self.screen
            .as_ref()?
            .items
            .as_ref()?
            .iter()
            .flat_map(|item| item.attr_list.as_ref().into_iter().flatten())
            .find(|attr| {
                attr.label
                    .as_deref()
                    .map(|l| {
                        l.contains("Артикул")
                            || l.contains("артикул")
                            || l.contains("Код товара")
                    })
                    .unwrap_or(false)
            })
            .and_then(|attr| attr.value.clone())
    }
}

const CRPT_API_BASE_URL: &str = "https://mobile.api.crpt.ru";
