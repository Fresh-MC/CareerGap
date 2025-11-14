use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub platform: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reversible: Option<bool>,
    pub check_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_glob: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_regex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_state: Option<ExpectedState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediate_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_value: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediate_params: Option<RemediateParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chmod_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_reboot_required: Option<bool>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: None,
            description: None,
            platform: String::new(),
            severity: None,
            reversible: None,
            check_type: String::new(),
            target_file: None,
            target_glob: None,
            regex: None,
            replace_regex: None,
            replace_with: None,
            key: None,
            expected_state: None,
            package_name: None,
            service_name: None,
            value_name: None,
            target_path: None,
            policy_name: None,
            right_name: None,
            port: None,
            protocol: None,
            remediate_type: None,
            value: None,
            set_value: None,
            set_type: None,
            remediate_params: None,
            chmod_mode: None,
            reference: None,
            post_reboot_required: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExpectedState {
    String(String),
    Map {
        operator: String,
        value: serde_yaml::Value,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemediateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
}
