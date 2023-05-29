static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

use std::collections::HashMap;
use strfmt::strfmt;

use base64::{engine::general_purpose as b64, Engine as _};
use reqwest::header::{self, HeaderValue};

use crate::NxCliConfig;
extern crate strfmt;

pub struct ApiClient {
    config: NxCliConfig,
}

impl ApiClient {
    pub fn create(nxconfig: NxCliConfig) -> Self {
        ApiClient { config: nxconfig }
    }

    pub fn get_config(&self) -> NxCliConfig {
        self.config.to_owned()
    }

    pub fn get_client(&self) -> reqwest::Client {
        // "Basic {auth}" auth=base64("{user}:{pass}")
        let mut headers = header::HeaderMap::new();
        let user = &self.config.user;
        let app_password = &self.config.app_password;
        let auth = b64::STANDARD_NO_PAD.encode(format!("{user}:{app_password}"));
        let mut auth_val = HeaderValue::from_str(format!("Basic {auth}").as_str()).unwrap();
        auth_val.set_sensitive(true);
        headers.append(header::AUTHORIZATION, auth_val);

        reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .default_headers(headers)
            .build()
            .unwrap()
    }

    pub fn build_url(&self, url: impl Into<String>) -> String {
        let mut vars = HashMap::new();
        vars.insert("user".to_string(), self.config.user.to_owned());

        let urlpart = strfmt(&url.into(), &vars).unwrap();

        let base = &self.config.server;
        format!("{base}{urlpart}")
    }
}
