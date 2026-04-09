use peekoo_plugin_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::crypto;
use crate::error::MijiaError;

const API_BASE: &str = "https://api.mijia.tech/app";
const SERVICE_LOGIN_URL: &str = "https://account.xiaomi.com/pass/serviceLogin";
const LOGIN_URL: &str = "https://account.xiaomi.com/longPolling/loginUrl";
const AUTH_STATE_KEY: &str = "mijia-auth";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(non_snake_case)]
pub struct AuthData {
    pub ssecurity: String,
    pub userId: String,
    pub cUserId: String,
    pub serviceToken: String,
    pub passToken: String,
    pub nonce: String,
    pub psecurity: String,
    pub deviceId: String,
    pub ua: String,
    pub pass_o: String,
    #[serde(default)]
    pub expireTime: i64,
    #[serde(default)]
    pub saveTime: i64,
}

impl AuthData {
    pub fn is_valid(&self) -> bool {
        !self.ssecurity.is_empty()
            && !self.userId.is_empty()
            && !self.cUserId.is_empty()
            && !self.serviceToken.is_empty()
            && !self.ua.is_empty()
    }
}

pub struct MijiaApi {
    auth: AuthData,
    locale: String,
}

impl MijiaApi {
    pub fn load() -> Result<Self, MijiaError> {
        let auth = load_auth_data()?;
        Ok(Self {
            auth,
            locale: "CN".to_string(),
        })
    }

    pub fn is_authenticated(&self) -> bool {
        self.auth.is_valid()
    }

    pub fn auth_path_display(&self) -> &str {
        "~/.peekoo/mijia"
    }

    fn ensure_device_fields(&mut self) {
        if self.auth.deviceId.is_empty() {
            let uuid = peekoo::system::uuid_v4().unwrap_or_default();
            self.auth.deviceId = uuid[..16.min(uuid.len())].to_string();
        }
        if self.auth.pass_o.is_empty() {
            let uuid = peekoo::system::uuid_v4().unwrap_or_default();
            self.auth.pass_o = uuid[..16.min(uuid.len())].to_string();
        }
        if self.auth.ua.is_empty() {
            let id1 = peekoo::system::uuid_v4().unwrap_or_default();
            let id2 = peekoo::system::uuid_v4().unwrap_or_default();
            let id3 = peekoo::system::uuid_v4().unwrap_or_default();
            let id4 = peekoo::system::uuid_v4().unwrap_or_default();
            self.auth.ua = format!(
                "Android-15-11.0.701-Xiaomi-23046RP50C-OS2.0.212.0.VMYCNXM-{}-CN-{}-{}-SmartHome-MI_APP_STORE-{}|{}|{}-64",
                &id1[..40.min(id1.len())],
                &id3[..32.min(id3.len())],
                &id2[..32.min(id2.len())],
                &id1[..40.min(id1.len())],
                &id4[..40.min(id4.len())],
                &self.auth.pass_o,
            );
        }
    }

    fn user_agent(&self) -> &str {
        &self.auth.ua
    }

    fn cookie_header(&self) -> String {
        format!(
            "cUserId={cuid};yetAnotherServiceToken={st};serviceToken={st};\
             timezone_id=Asia/Shanghai;timezone=GMT+08:00;is_daylight=0;\
             dst_offset=0;channel=MI_APP_STORE;countryCode={cc};\
             PassportDeviceId={did};locale={locale}",
            cuid = self.auth.cUserId,
            st = self.auth.serviceToken,
            cc = self.locale,
            did = self.auth.deviceId,
            locale = format!("zh_{}", self.locale),
        )
    }

    fn service_login_url(&self) -> String {
        format!(
            "{SERVICE_LOGIN_URL}?_json=true&sid=mijia&_locale={locale}",
            locale = format!("zh_{}", self.locale),
        )
    }

    fn device_id_cookie(&self) -> String {
        format!(
            "deviceId={did};pass_o={po};passToken={pt};userId={uid};cUserId={cuid};uLocale={loc}",
            did = self.auth.deviceId,
            po = self.auth.pass_o,
            pt = self.auth.passToken,
            uid = self.auth.userId,
            cuid = self.auth.cUserId,
            loc = format!("zh_{}", self.locale),
        )
    }

    /// Core authenticated request: signs params, RC4-encrypts, POSTs, decrypts response.
    pub fn request(&self, uri: &str, data: &Value) -> Result<Value, MijiaError> {
        let url = format!("{API_BASE}{uri}");
        let data_json = serde_json::to_string(data)
            .map_err(|e| MijiaError::Parse(format!("data serialize: {e}")))?;

        let nonce = crypto::gen_nonce();
        let signed_nonce = crypto::get_signed_nonce(&self.auth.ssecurity, &nonce)?;

        let params = crypto::build_enc_params(
            uri,
            "POST",
            &signed_nonce,
            &nonce,
            &self.auth.ssecurity,
            &data_json,
        )?;

        let body = crypto::encode_form_params(&params);

        let cookie = self.cookie_header();
        let ua = self.user_agent().to_string();

        let response = peekoo::http::request(peekoo::http::Request {
            method: "POST",
            url: &url,
            headers: vec![
                ("Content-Type", "application/x-www-form-urlencoded"),
                ("User-Agent", &ua),
                ("accept-encoding", "identity"),
                ("Cookie", &cookie),
            ],
            body: Some(&body),
        })
        .map_err(|e| MijiaError::Http {
            status: 0,
            body: e.to_string(),
        })?;

        if response.status >= 400 {
            return Err(MijiaError::Http {
                status: response.status,
                body: response.body,
            });
        }

        let text = response.body;

        // Try parse as JSON directly
        let parsed: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => {
                // Response is RC4-encrypted, decrypt it
                let decrypted = crypto::decrypt_response(&self.auth.ssecurity, &nonce, &text)?;
                serde_json::from_str(&decrypted)
                    .map_err(|e| MijiaError::Parse(format!("decrypt parse: {e}")))?
            }
        };

        let code = parsed["code"].as_i64().unwrap_or(-1);
        if code != 0 && !parsed.get("result").is_some() {
            return Err(MijiaError::Api {
                code,
                message: parsed["message"]
                    .as_str()
                    .or_else(|| parsed["desc"].as_str())
                    .unwrap_or("未知错误")
                    .to_string(),
            });
        }

        Ok(parsed["result"].clone())
    }

    // ── Auth Flow ──────────────────────────────────────────────────

    /// Check if token is still valid by calling the check_new_msg endpoint.
    #[allow(dead_code)]
    pub fn check_token(&self) -> bool {
        let data = json!({"begin_at": 0});
        self.request("/v2/message/v2/check_new_msg", &data).is_ok()
    }

    /// Step 1 of QR login: get the QR URL and long-poll URL.
    pub fn login_start(&mut self) -> Result<Value, MijiaError> {
        self.ensure_device_fields();

        // First try to refresh token via serviceLogin
        let url = self.service_login_url();
        let ua = self.user_agent().to_string();
        let device_cookie = self.device_id_cookie();

        let response = peekoo::http::request(peekoo::http::Request {
            method: "GET",
            url: &url,
            headers: vec![
                ("User-Agent", &ua),
                ("Connection", "keep-alive"),
                ("Content-Type", "application/x-www-form-urlencoded"),
                ("Cookie", &device_cookie),
            ],
            body: None,
        })
        .map_err(|e| MijiaError::Http {
            status: 0,
            body: e.to_string(),
        })?;

        if response.status >= 400 {
            return Err(MijiaError::Http {
                status: response.status,
                body: response.body,
            });
        }

        let service_data = parse_service_response(&response.body)?;

        // If code == 0, token is still valid
        if service_data["code"].as_i64().unwrap_or(-1) == 0 {
            return Ok(json!({
                "success": true,
                "authenticated": true,
                "message": "Token is still valid. No need to scan again."
            }));
        }

        // Get login URL params from service_data["location"]
        let location = service_data["location"]
            .as_str()
            .ok_or_else(|| MijiaError::Login("missing location".into()))?;
        let mut location_params = parse_url_query(location);

        location_params.insert("theme".into(), "".into());
        location_params.insert("bizDeviceType".into(), "".into());
        location_params.insert("_hasLogo".into(), "false".into());
        location_params.insert("_qrsize".into(), "240".into());
        let dc = peekoo::system::time_millis().unwrap_or(0).to_string();
        location_params.insert("_dc".into(), dc);

        let login_query = location_params
            .iter()
            .map(|(k, v)| format!("{}={}", crypto::url_encode(k), crypto::url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let login_full_url = format!("{LOGIN_URL}?{login_query}");

        let login_response = peekoo::http::request(peekoo::http::Request {
            method: "GET",
            url: &login_full_url,
            headers: vec![
                ("User-Agent", &ua),
                ("Content-Type", "application/x-www-form-urlencoded"),
                ("Connection", "keep-alive"),
            ],
            body: None,
        })
        .map_err(|e| MijiaError::Http {
            status: 0,
            body: e.to_string(),
        })?;

        if login_response.status >= 400 {
            return Err(MijiaError::Http {
                status: login_response.status,
                body: login_response.body,
            });
        }

        let login_data = parse_service_response(&login_response.body)?;

        let qr_url = login_data["qr"].as_str().unwrap_or_default().to_string();
        let login_url = login_data["loginUrl"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let lp_url = login_data["lp"].as_str().unwrap_or_default().to_string();

        peekoo::log::info(&format!(
            "login_start: qr_len={}, lp_len={}",
            qr_url.len(),
            lp_url.len()
        ));

        if lp_url.is_empty() {
            peekoo::log::error("login_start: lp_url is empty in response");
            return Err(MijiaError::Login(
                "Login response missing long-poll URL".into(),
            ));
        }

        // Save pending login state (persist device identity across login_start -> login_finish)
        let pending = json!({
            "lp": lp_url,
            "ua": self.auth.ua,
            "deviceId": self.auth.deviceId,
            "pass_o": self.auth.pass_o,
            "headers": {
                "User-Agent": ua,
                "Content-Type": "application/x-www-form-urlencoded",
                "Connection": "keep-alive",
            },
            "created_at": (peekoo::system::time_millis().unwrap_or(0) / 1000) as i64,
        });
        let _ = peekoo::state::set(
            "mijia-login-pending",
            &serde_json::to_string(&pending).unwrap_or_default(),
        );

        Ok(json!({
            "success": true,
            "authenticated": false,
            "needs_scan": true,
            "qr_url": qr_url,
            "login_url": login_url,
            "message": "Scan the QR code in Mijia app and confirm sign-in",
        }))
    }

    /// Step 2 of QR login: poll the long-poll URL and complete auth.
    pub fn login_finish(&mut self, _timeout_secs: i64) -> Result<Value, MijiaError> {
        self.ensure_device_fields();

        let pending_raw: Option<String> = peekoo::state::get("mijia-login-pending").ok().flatten();
        let pending_str = pending_raw
            .ok_or_else(|| MijiaError::Login("No pending login session found".into()))?;

        peekoo::log::info(&format!(
            "login_finish: pending state length={}",
            pending_str.len()
        ));

        let pending: Value = serde_json::from_str(&pending_str)
            .map_err(|e| MijiaError::Parse(format!("pending parse: {e}")))?;

        let lp_url = pending["lp"]
            .as_str()
            .ok_or_else(|| MijiaError::Login("missing lp URL in pending state".into()))?;

        peekoo::log::info(&format!(
            "login_finish: lp_url={}",
            &lp_url[..lp_url.len().min(80)]
        ));

        if lp_url.is_empty() {
            return Err(MijiaError::Login("lp URL is empty".into()));
        }

        // Validate URL format
        if !lp_url.starts_with("https://") && !lp_url.starts_with("http://") {
            peekoo::log::error(&format!(
                "login_finish: lp_url has invalid scheme: {}",
                &lp_url[..lp_url.len().min(40)]
            ));
            return Err(MijiaError::Login(format!(
                "lp URL has invalid format: {}",
                &lp_url[..lp_url.len().min(40)]
            )));
        }

        let ua = pending["headers"]["User-Agent"]
            .as_str()
            .unwrap_or("Peekoo-Desktop/0.1.0");

        // Restore device identity from login_start so requests use the same UA/deviceId
        if let Some(saved_ua) = pending["ua"].as_str() {
            self.auth.ua = saved_ua.to_string();
        }
        if let Some(did) = pending["deviceId"].as_str() {
            self.auth.deviceId = did.to_string();
        }
        if let Some(po) = pending["pass_o"].as_str() {
            self.auth.pass_o = po.to_string();
        }

        // Long-poll: user scans QR and confirms
        peekoo::log::info("login_finish: sending long-poll request");
        let lp_result = peekoo::http::request(peekoo::http::Request {
            method: "GET",
            url: lp_url,
            headers: vec![("User-Agent", ua), ("Connection", "keep-alive")],
            body: None,
        });

        match lp_result {
            Ok(lp_response) => {
                peekoo::log::info(&format!(
                    "login_finish: long-poll response status={}",
                    lp_response.status
                ));
                if lp_response.status < 400 {
                    // Collect cookies from long-poll response to forward to callback
                    let lp_cookies = cookies_from_headers(&lp_response.headers);
                    let lp_data = parse_service_response(&lp_response.body)?;
                    self.apply_login_response(&lp_data, &lp_cookies)?;
                    save_auth_data(&self.auth)?;
                    let _ = peekoo::state::set("mijia-login-pending", &String::new());
                    return Ok(json!({
                        "success": true,
                        "authenticated": true,
                        "message": "Sign-in successful",
                    }));
                }
                peekoo::log::warn(&format!(
                    "login_finish: long-poll returned {}, trying serviceLogin fallback",
                    lp_response.status
                ));
            }
            Err(e) => {
                peekoo::log::warn(&format!(
                    "login_finish: long-poll failed ({e}), trying serviceLogin fallback"
                ));
            }
        }

        // Fallback: use serviceLogin to refresh token after user confirmed
        self.try_service_login_refresh(ua)
    }

    fn apply_login_response(
        &mut self,
        lp_data: &Value,
        lp_cookies: &str,
    ) -> Result<(), MijiaError> {
        self.auth.psecurity = lp_data["psecurity"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        self.auth.nonce = lp_data["nonce"].as_str().unwrap_or_default().to_string();
        self.auth.ssecurity = lp_data["ssecurity"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        self.auth.passToken = lp_data["passToken"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        self.auth.userId = lp_data["userId"].as_str().unwrap_or_default().to_string();
        self.auth.cUserId = lp_data["cUserId"].as_str().unwrap_or_default().to_string();

        // Follow callback URL to get serviceToken cookie
        let callback_url = lp_data["location"]
            .as_str()
            .ok_or_else(|| MijiaError::Login("missing callback location".into()))?;

        let ua_str = &self.auth.ua;
        let mut headers = vec![
            ("User-Agent", ua_str.as_str()),
            ("Connection", "keep-alive"),
        ];
        if !lp_cookies.is_empty() {
            headers.push(("Cookie", lp_cookies));
        }

        let cb_response = peekoo::http::request(peekoo::http::Request {
            method: "GET",
            url: callback_url,
            headers,
            body: None,
        })
        .map_err(|e| MijiaError::Http {
            status: 0,
            body: e.to_string(),
        })?;

        for (key, val) in &cb_response.headers {
            if key.to_lowercase() == "set-cookie" {
                if let Some(token) = extract_cookie_value(val, "serviceToken") {
                    self.auth.serviceToken = token;
                }
            }
        }

        self.auth.expireTime =
            (peekoo::system::time_millis().unwrap_or(0) as i64) + 30 * 24 * 3600 * 1000;
        Ok(())
    }

    fn try_service_login_refresh(&mut self, ua: &str) -> Result<Value, MijiaError> {
        peekoo::log::info("login_finish: attempting serviceLogin refresh");
        let url = self.service_login_url();
        let device_cookie = self.device_id_cookie();

        let response = peekoo::http::request(peekoo::http::Request {
            method: "GET",
            url: &url,
            headers: vec![
                ("User-Agent", ua),
                ("Connection", "keep-alive"),
                ("Content-Type", "application/x-www-form-urlencoded"),
                ("Cookie", &device_cookie),
            ],
            body: None,
        })
        .map_err(|e| MijiaError::Http {
            status: 0,
            body: e.to_string(),
        })?;

        if response.status >= 400 {
            return Err(MijiaError::Login(format!(
                "serviceLogin refresh failed ({}): {}",
                response.status, response.body
            )));
        }

        let service_data = parse_service_response(&response.body)?;
        peekoo::log::info(&format!(
            "login_finish: serviceLogin code={}",
            service_data["code"].as_i64().unwrap_or(-1)
        ));

        if service_data["code"].as_i64().unwrap_or(-1) == 0 {
            // User confirmed on phone — follow location to get cookies
            let location = service_data["location"]
                .as_str()
                .ok_or_else(|| MijiaError::Login("missing location in serviceLogin".into()))?;

            self.auth.ssecurity = service_data["ssecurity"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            let cb_response = peekoo::http::request(peekoo::http::Request {
                method: "GET",
                url: location,
                headers: vec![("User-Agent", ua), ("Connection", "keep-alive")],
                body: None,
            })
            .map_err(|e| MijiaError::Http {
                status: 0,
                body: e.to_string(),
            })?;

            // Extract cookies from Set-Cookie headers
            for (key, val) in &cb_response.headers {
                if key.to_lowercase() == "set-cookie" {
                    if let Some(token) = extract_cookie_value(val, "serviceToken") {
                        self.auth.serviceToken = token;
                    }
                    if let Some(token) = extract_cookie_value(val, "passToken") {
                        self.auth.passToken = token;
                    }
                    if let Some(uid) = extract_cookie_value(val, "userId") {
                        self.auth.userId = uid;
                    }
                    if let Some(cuid) = extract_cookie_value(val, "cUserId") {
                        self.auth.cUserId = cuid;
                    }
                }
            }

            self.auth.expireTime =
                (peekoo::system::time_millis().unwrap_or(0) as i64) + 30 * 24 * 3600 * 1000;

            save_auth_data(&self.auth)?;
            let _ = peekoo::state::set("mijia-login-pending", &String::new());

            return Ok(json!({
                "success": true,
                "authenticated": true,
                "message": "Sign-in successful",
            }));
        }

        Err(MijiaError::Login(
            "Login not confirmed yet. Please scan the QR code in Mijia app and confirm.".into(),
        ))
    }

    /// Logout: clear auth data.
    pub fn logout() -> Result<Value, MijiaError> {
        let _ = peekoo::state::set(AUTH_STATE_KEY, &String::new());
        let _ = peekoo::state::set("mijia-login-pending", &String::new());
        Ok(json!({
            "success": true,
            "authenticated": false,
            "logged_out": true,
            "message": "Signed out successfully",
        }))
    }

    // ── API Endpoints ──────────────────────────────────────────────

    pub fn get_homes_list(&self) -> Result<Value, MijiaError> {
        let data = json!({
            "fg": true,
            "fetch_share": true,
            "fetch_share_dev": true,
            "fetch_cariot": true,
            "limit": 300,
            "app_ver": 7,
            "plat_form": 0
        });
        let result = self.request("/v2/homeroom/gethome_merged", &data)?;
        Ok(result["homelist"].clone())
    }

    pub fn get_devices_list(&self, home_id: &str) -> Result<Value, MijiaError> {
        let owner = self.get_home_owner(home_id)?;
        let mut devices = Vec::new();
        let mut start_did = String::new();
        let mut has_more = true;

        while has_more {
            let data = json!({
                "home_owner": owner,
                "home_id": home_id.parse::<i64>().unwrap_or(0),
                "limit": 200,
                "start_did": start_did,
                "get_split_device": true,
                "support_smart_home": true,
                "get_cariot_device": true,
                "get_third_device": true
            });
            let ret = self.request("/home/home_device_list", &data)?;
            if let Some(info_list) = ret["device_info"].as_array() {
                for dev in info_list {
                    let mut d = dev.clone();
                    d["home_id"] = json!(home_id);
                    devices.push(d);
                }
                start_did = ret["max_did"].as_str().unwrap_or("").to_string();
                has_more = ret["has_more"].as_bool().unwrap_or(false) && !start_did.is_empty();
            } else {
                has_more = false;
            }
        }

        Ok(Value::Array(devices))
    }

    pub fn get_all_devices(&self) -> Result<Value, MijiaError> {
        let homes = self.get_homes_list()?;
        let home_list = homes.as_array().cloned().unwrap_or_default();
        let mut all_devices = Vec::new();

        for home in &home_list {
            let home_id = home["id"].to_string().trim_matches('"').to_string();
            let devices = self.get_devices_list(&home_id)?;
            if let Some(arr) = devices.as_array() {
                all_devices.extend(arr.clone());
            }
        }

        Ok(Value::Array(all_devices))
    }

    pub fn get_shared_devices_list(&self) -> Result<Value, MijiaError> {
        let data = json!({
            "ssid": "<unknown ssid>",
            "bssid": "02:00:00:00:00:00",
            "getVirtualModel": true,
            "getHuamiDevices": 1,
            "get_split_device": true,
            "support_smart_home": true,
            "get_cariot_device": true,
            "get_third_device": true,
            "get_phone_device": true,
            "get_miwear_device": true
        });
        let ret = self.request("/v2/home/device_list_page", &data)?;
        let mut devices = Vec::new();
        if let Some(list) = ret["list"].as_array() {
            for item in list {
                if item["owner"].as_bool().unwrap_or(false) {
                    let mut d = item.clone();
                    d["home_id"] = json!("shared");
                    devices.push(d);
                }
            }
        }
        Ok(Value::Array(devices))
    }

    pub fn get_devices_prop(&self, queries: &Value) -> Result<Value, MijiaError> {
        let params = if queries.is_array() {
            queries.clone()
        } else {
            json!([queries])
        };
        let data = json!({ "params": params, "datasource": 1 });
        let ret = self.request("/miotspec/prop/get", &data)?;

        if queries.is_object() {
            if let Some(arr) = ret.as_array() {
                if arr.len() == 1 {
                    return Ok(arr[0].clone());
                }
            }
        }
        Ok(ret)
    }

    pub fn set_devices_prop(&self, queries: &Value) -> Result<Value, MijiaError> {
        let params = if queries.is_array() {
            queries.clone()
        } else {
            json!([queries])
        };
        let data = json!({ "params": params });
        let mut ret = self.request("/miotspec/prop/set", &data)?;

        if let Some(arr) = ret.as_array_mut() {
            for item in arr.iter_mut() {
                let code = item["code"].as_i64().unwrap_or(0);
                if code != 0 && code != 1 {
                    item["message"] = json!(crate::error::error_message(code));
                } else {
                    item["message"] = json!("成功");
                }
            }
        }

        if queries.is_object() {
            if let Some(arr) = ret.as_array() {
                if arr.len() == 1 {
                    return Ok(arr[0].clone());
                }
            }
        }
        Ok(ret)
    }

    pub fn run_action(&self, queries: &Value) -> Result<Value, MijiaError> {
        let params = if queries.is_array() {
            queries.clone()
        } else {
            json!([queries])
        };

        let mut results = Vec::new();
        if let Some(arr) = params.as_array() {
            for param in arr {
                let data = json!({ "params": param });
                let ret = self.request("/miotspec/action", &data)?;
                results.push(ret);
            }
        }

        for item in &mut results {
            let code = item["code"].as_i64().unwrap_or(0);
            if code != 0 && code != 1 {
                item["message"] = json!(crate::error::error_message(code));
            } else {
                item["message"] = json!("成功");
            }
        }

        if queries.is_object() && results.len() == 1 {
            return Ok(results.remove(0));
        }
        Ok(Value::Array(results))
    }

    pub fn get_statistics(&self, params: &Value) -> Result<Value, MijiaError> {
        if params.is_array() {
            let mut results = Vec::new();
            for p in params.as_array().unwrap() {
                let ret = self.request("/v2/user/statistics", p)?;
                results.push(ret);
            }
            Ok(Value::Array(results))
        } else {
            self.request("/v2/user/statistics", params)
        }
    }

    fn get_home_owner(&self, home_id: &str) -> Result<i64, MijiaError> {
        let homes = self.get_homes_list()?;
        if let Some(arr) = homes.as_array() {
            for home in arr {
                if home["id"].to_string().trim_matches('"') == home_id {
                    return Ok(home["uid"].as_i64().unwrap_or(0));
                }
            }
        }
        Err(MijiaError::Api {
            code: -1,
            message: format!("未找到 home_id={home_id} 的家庭信息"),
        })
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn load_auth_data() -> Result<AuthData, MijiaError> {
    let raw: Option<String> = peekoo::state::get(AUTH_STATE_KEY)
        .map_err(|e| MijiaError::Parse(format!("state read: {e}")))?;

    match raw {
        Some(json_str) if !json_str.is_empty() => serde_json::from_str(&json_str)
            .map_err(|e| MijiaError::Parse(format!("auth parse: {e}"))),
        _ => Ok(AuthData::default()),
    }
}

pub fn save_auth_data(auth: &AuthData) -> Result<(), MijiaError> {
    let json_str = serde_json::to_string(auth)
        .map_err(|e| MijiaError::Parse(format!("auth serialize: {e}")))?;
    peekoo::state::set(AUTH_STATE_KEY, &json_str)
        .map_err(|e| MijiaError::Parse(format!("state write: {e}")))?;
    Ok(())
}

fn parse_service_response(text: &str) -> Result<Value, MijiaError> {
    let cleaned = text.replace("&&&START&&&", "");
    serde_json::from_str(&cleaned).map_err(|e| MijiaError::Parse(format!("service response: {e}")))
}

fn parse_url_query(url: &str) -> std::collections::HashMap<String, String> {
    let mut params = std::collections::HashMap::new();
    if let Some(query_start) = url.find('?') {
        let query = &url[query_start + 1..];
        for pair in query.split('&') {
            if let Some(eq_pos) = pair.find('=') {
                let key = &pair[..eq_pos];
                let val = &pair[eq_pos + 1..];
                params.insert(url_decode(key), url_decode(val));
            }
        }
    }
    params
}

fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                result.push((hi * 16 + lo) as char);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            result.push(' ');
            i += 1;
            continue;
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn extract_cookie_value(set_cookie: &str, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    for part in set_cookie.split(';') {
        let trimmed = part.trim();
        if let Some(val) = trimmed.strip_prefix(&prefix) {
            return Some(val.to_string());
        }
    }
    None
}

/// Build a `Cookie` header string from a response's `Set-Cookie` headers.
fn cookies_from_headers(headers: &[(String, String)]) -> String {
    let mut parts = Vec::new();
    for (name, value) in headers {
        if name.to_lowercase() == "set-cookie" {
            // Each Set-Cookie: name=value; Path=/; HttpOnly → take "name=value"
            if let Some(name_value) = value.split(';').next() {
                let trimmed = name_value.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
        }
    }
    parts.join("; ")
}
