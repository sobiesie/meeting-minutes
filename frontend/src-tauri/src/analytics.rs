use posthog_rs::{Client, Event};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    pub api_key: String,
    pub host: Option<String>,
    pub enabled: bool,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            host: Some("https://us.i.posthog.com".to_string()),
            enabled: false,
        }
    }
}

pub struct AnalyticsClient {
    client: Option<Arc<Client>>,
    config: AnalyticsConfig,
    user_id: Arc<Mutex<Option<String>>>,
}

impl AnalyticsClient {
    pub async fn new(config: AnalyticsConfig) -> Self {
        let client = if config.enabled && !config.api_key.is_empty() {
            Some(Arc::new(posthog_rs::client(config.api_key.as_str()).await))
        } else {
            None
        };

        Self {
            client,
            config,
            user_id: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn identify(&self, user_id: String, properties: Option<HashMap<String, String>>) -> Result<(), String> {
        let client = match &self.client {
            Some(client) => Arc::clone(client),
            None => return Ok(()),
        };

        // Store user ID for future events
        *self.user_id.lock().await = Some(user_id.clone());

                let properties = properties.unwrap_or_default();
        
        let mut event = Event::new("$identify", &user_id);
        
        // Add user properties
        for (key, value) in properties {
            if let Err(e) = event.insert_prop(&key, value) {
                eprintln!("Failed to add property {}: {}", key, e);
            }
        }
        
        if let Err(e) = client.capture(event).await {
            eprintln!("Failed to identify user: {}", e);
        }
        
        Ok(())
    }

    pub async fn track_event(&self, event_name: &str, properties: Option<HashMap<String, String>>) -> Result<(), String> {
        let client = match &self.client {
            Some(client) => Arc::clone(client),
            None => return Ok(()),
        };

        let user_id = self.user_id.lock().await.clone()
            .unwrap_or_else(|| format!("anonymous_{}", Uuid::new_v4()));

                let event_name = event_name.to_string();
        let properties = properties.unwrap_or_default();
        
        let mut event = Event::new(&event_name, &user_id);
        
        // Add event properties
        for (key, value) in properties {
            if let Err(e) = event.insert_prop(&key, value) {
                eprintln!("Failed to add property {}: {}", key, e);
            }
        }
        
        if let Err(e) = client.capture(event).await {
            eprintln!("Failed to track event {}: {}", event_name, e);
        }
        
        Ok(())
    }

    // Meeting-specific event tracking methods
    pub async fn track_meeting_started(&self, meeting_id: &str, meeting_title: &str) -> Result<(), String> {
        let mut properties = HashMap::new();
        properties.insert("meeting_id".to_string(), meeting_id.to_string());
        properties.insert("meeting_title".to_string(), meeting_title.to_string());
        properties.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        
        self.track_event("meeting_started", Some(properties)).await
    }

    pub async fn track_recording_started(&self, meeting_id: &str) -> Result<(), String> {
        let mut properties = HashMap::new();
        properties.insert("meeting_id".to_string(), meeting_id.to_string());
        properties.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        
        self.track_event("recording_started", Some(properties)).await
    }

    pub async fn track_recording_stopped(&self, meeting_id: &str, duration_seconds: Option<u64>) -> Result<(), String> {
        let mut properties = HashMap::new();
        properties.insert("meeting_id".to_string(), meeting_id.to_string());
        properties.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        
        if let Some(duration) = duration_seconds {
            properties.insert("duration_seconds".to_string(), duration.to_string());
        }
        
        self.track_event("recording_stopped", Some(properties)).await
    }

    pub async fn track_meeting_deleted(&self, meeting_id: &str) -> Result<(), String> {
        let mut properties = HashMap::new();
        properties.insert("meeting_id".to_string(), meeting_id.to_string());
        properties.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        
        self.track_event("meeting_deleted", Some(properties)).await
    }

    pub async fn track_search_performed(&self, query: &str, results_count: usize) -> Result<(), String> {
        let mut properties = HashMap::new();
        properties.insert("query".to_string(), query.to_string());
        properties.insert("results_count".to_string(), results_count.to_string());
        properties.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        
        self.track_event("search_performed", Some(properties)).await
    }

    pub async fn track_settings_changed(&self, setting_type: &str, new_value: &str) -> Result<(), String> {
        let mut properties = HashMap::new();
        properties.insert("setting_type".to_string(), setting_type.to_string());
        properties.insert("new_value".to_string(), new_value.to_string());
        properties.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        
        self.track_event("settings_changed", Some(properties)).await
    }

    pub async fn track_app_started(&self, version: &str) -> Result<(), String> {
        let mut properties = HashMap::new();
        properties.insert("app_version".to_string(), version.to_string());
        properties.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        
        self.track_event("app_started", Some(properties)).await
    }

    pub async fn track_feature_used(&self, feature_name: &str) -> Result<(), String> {
        let mut properties = HashMap::new();
        properties.insert("feature_name".to_string(), feature_name.to_string());
        properties.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        
        self.track_event("feature_used", Some(properties)).await
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.client.is_some()
    }

    pub async fn set_user_properties(&self, properties: HashMap<String, String>) -> Result<(), String> {
        let client = match &self.client {
            Some(client) => Arc::clone(client),
            None => return Ok(()),
        };

                let user_id = self.user_id.lock().await.clone()
            .unwrap_or_else(|| format!("anonymous_{}", Uuid::new_v4()));
        
        let mut event = Event::new("$set", &user_id);
        
        // Add user properties
        for (key, value) in properties {
            if let Err(e) = event.insert_prop(&key, value) {
                eprintln!("Failed to add property {}: {}", key, e);
            }
        }
        
        if let Err(e) = client.capture(event).await {
            eprintln!("Failed to set user properties: {}", e);
        }
        
        Ok(())
    }
}

// Helper function to create analytics client from config
pub async fn create_analytics_client(config: AnalyticsConfig) -> AnalyticsClient {
    AnalyticsClient::new(config).await
} 