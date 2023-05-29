const APP_NAME: &str = "nxcli";

use self::apps::calendar::get_calendar_list;
use crate::api::ApiClient;
use crate::apps::todo::get_todos;
use reqwest::header::USER_AGENT;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{thread, time::Duration};

mod api;
mod apps;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct NxCliConfig {
    server: String,
    #[serde(alias = "loginName")]
    user: String,
    #[serde(alias = "appPassword")]
    app_password: String,
}

impl NxCliConfig {
    fn is_empty(&self) -> bool {
        self.app_password.is_empty() || self.user.is_empty() || self.server.is_empty()
    }

    fn load() -> Self {
        confy::load(APP_NAME, None).unwrap_or_default()
    }

    fn save(&self) -> bool {
        confy::store(APP_NAME, None, self).is_ok()
    }
}

fn get_config() -> Option<NxCliConfig> {
    let loaded_cfg = NxCliConfig::load();
    if !loaded_cfg.is_empty() {
        return Some(loaded_cfg);
    }

    if loaded_cfg.is_empty() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        match rt.block_on(ask_login()) {
            Ok(config) => return Some(config),
            Err(_) => return None,
        }
    }

    None
}

fn main() {
    let cfg_option = get_config();

    if let Some(config) = cfg_option {
        let api = ApiClient::create(config);
        let rt = tokio::runtime::Runtime::new().unwrap();

        let calendars = rt.block_on(get_calendar_list(&api));
        for cal in calendars {
            if let Some(todos) = rt.block_on(get_todos(&api, &cal)) {
                dbg!(todos);
                break;
            }
        }
        // dbg!(calendars);
    }

    println!("Hello, world!");
}

#[derive(Debug, Deserialize)]
struct NextCloudLoginPoll {
    token: String,
    endpoint: String,
}

#[derive(Debug, Deserialize)]
struct NextCloudLogin {
    poll: NextCloudLoginPoll,
    login: String,
}

async fn ask_login() -> Result<NxCliConfig, String> {
    println!("Ask Login");

    let server = prompt("Nextcloud Server URL > ");

    let fqdn: String = if server.starts_with("https://") || server.starts_with("http://") {
        server
    } else {
        format!("https://{}", server)
    };

    let client = reqwest::Client::new();

    let response = client
        .post(fqdn + "/index.php/login/v2")
        .header(USER_AGENT, APP_NAME)
        .send()
        .await
        .unwrap();

    let data: NextCloudLogin = response.json().await.unwrap();

    println!("Please Open: {}", data.login);

    let mut check_counter = 0;

    // loop until there is a 200 answer
    // 200 answer means user did grant access
    // 404 means user did not yet login/granted access
    // everythign else - we dont know
    // see https://docs.nextcloud.com/server/latest/developer_manual/client_apis/LoginFlow/index.html#login-flow-v2
    loop {
        let wait_time = match check_counter {
            1..=10 => 1,
            11..=30 => 2,
            31..=90 => 4,
            _ => 10,
        };

        let response = client
            .post(data.poll.endpoint.clone())
            .form(&[("token", data.poll.token.clone())])
            .send()
            .await
            .unwrap();

        match response.status() {
            StatusCode::NOT_FOUND => {
                print!(".");
                std::io::stdout().flush().unwrap();
                thread::sleep(Duration::from_secs(wait_time))
            }
            StatusCode::OK => {
                let login_data: NxCliConfig = response.json().await.unwrap();
                login_data.save();

                return Ok(login_data);
            }
            _ => {
                return Err("Strange Error".to_string());
            }
        }
        check_counter += 1;
    }
}

fn prompt(name: &str) -> String {
    let mut line = String::new();
    print!("{}", name);
    std::io::stdout().flush().unwrap();
    std::io::stdin()
        .read_line(&mut line)
        .expect("Error: Could not read a line");

    return line.trim().to_string();
}
