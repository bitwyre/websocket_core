use serde::Deserialize;
use serde::Serialize;
use serde_json;
use std::collections::HashMap;
use std::string::ToString;

pub trait JsonSerializable<'a, T = Self>
where
    Self: Deserialize<'a> + Serialize,
{
    #[inline]
    fn from_json(json_string: &'a str) -> Option<Self> {
        serde_json::from_str::<'a, Self>(json_string).ok()
    }

    #[inline]
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Clone)]
pub enum WebsocketServiceType {
    PeriodicBroadcast,
    PubSubBroadcast,
    Reactive,
}

#[derive(Serialize, Deserialize)]
pub struct CommonResponse {
    pub error: Vec<String>,
    pub result: HashMap<String, String>,
}

impl Default for CommonResponse {
    #[inline]
    fn default() -> Self {
        Self {
            error: Vec::default(),
            result: HashMap::default(),
        }
    }
}

impl JsonSerializable<'_> for CommonResponse {}

impl ToString for CommonResponse {
    #[inline]
    fn to_string(&self) -> String {
        self.to_json()
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_empty_common_response_is_as_expected() {
        const EXPECTED_STRING: &str = "{\"error\":[\"Some Error!\"],\"result\":{}}";
        let mut response = CommonResponse::default();
        response.error.push("Some Error!".to_owned());
        let json_response = response.to_string();
        assert_eq!(&json_response, EXPECTED_STRING);
    }

    #[test]
    fn test_json_serialization_back_and_forth_is_unique() {
        const ERROR_MESSAGE: &str = "Some Error!";
        let mut original_response = CommonResponse::default();
        original_response.error.push(ERROR_MESSAGE.to_owned());
        let json_response = original_response.to_string();
        let reconstructed_response = CommonResponse::from_json(&json_response).unwrap();
        assert!(reconstructed_response.error.contains(&ERROR_MESSAGE.to_owned()));
        assert!(&reconstructed_response as *const _ != &original_response as *const _);
    }
}
