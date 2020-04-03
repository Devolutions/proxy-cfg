use super::*;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use winapi::{shared::minwindef::*, um::winhttp::*, um::winnt::LPWSTR};
use winreg::RegKey;
use winreg::enums::*;

const REG_POLICIES: &str = r"Software\Policies\Microsoft\Windows\CurrentVersion\Internet Settings";
const REG_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
const REG_CONNECTIONS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings\Connections";

#[derive(PartialEq)]
enum AutoconfigType {
    Pac,
    Wpad,
    None,
}

unsafe fn lpwstr_null_to_string(wide: LPWSTR) -> Option<String> {
    if wide.is_null() {
        return None
    }

    let len = (0..).take_while(|&i| *wide.offset(i) != 0).count();
    let slice = std::slice::from_raw_parts(wide, len);
    OsString::from_wide(slice).into_string().ok()
}

// Bypass list is semi-colon or whitespace delimited
// The special value "<local>" means all local addresses
fn parse_bypass_list(bypass_list: &str) -> Vec<String> {
    bypass_list.split(&[' ', ';'][..])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.into())
        .collect()
}

// Proxy list is semi-colon or whitespace delimited, in this format:
// ([<scheme>=][<scheme>"://"]<server>[":"<port>])
fn parse_proxy_list(proxy_list: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    let proxies = proxy_list.split(&[' ', ';'][..])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    for proxy in proxies {
        let split: Vec<&str> = proxy.split('=').collect();

        if split.len() == 1 {
            result.insert("http".into(), split[0].into());
        } else if split.len() == 2 {
            result.insert(split[0].into(), split[1].into());
        }
    }

    result
}

fn win_inet_is_per_user() -> bool {
    if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(REG_POLICIES) {
        match key.get_value("ProxySettingsPerUser") {
            Ok(0u32) => return false,
            _ => return true,
        }
    };

    true
}

fn win_inet_get_autoconfig_type(connections: RegKey) -> AutoconfigType {
    if let Ok(default_connection_settings) = connections.get_raw_value("DefaultConnectionSettings") {
        let bytes = default_connection_settings.bytes;

        // Format of DefaultConnectionSettings is a string of bytes
        // Only interested in byte 9 here which values mean:
        //  09 when only 'Automatically detect settings' is enabled 
        //  03 when only 'Use a proxy server for your LAN' is enabled
        //  0B when both are enabled
        //  05 when only 'Use automatic configuration script' is enabled
        //  0D when 'Automatically detect settings' and 'Use automatic configuration script' are enabled
        //  07 when 'Use a proxy server for your LAN' and 'Use automatic configuration script' are enabled
        //  0F when all the three are enabled. 
        //  01 when none of them are enabled. 
        // Source https://superuser.com/questions/419696/in-windows-7-how-to-change-proxy-settings-from-command-line
        if bytes.len() > 8 {
            if (bytes[8] & (1 << 2)) == (1 << 2) {
                return AutoconfigType::Pac;
            } else if (bytes[8] & (1 << 3)) == (1 << 3) {
                return AutoconfigType::Wpad;
            }
        }
    }

    return AutoconfigType::None;
}

fn win_inet_get_proxy_config(internet_settings: RegKey) -> Option<ProxyConfig> {
    if internet_settings.get_value("ProxyEnable").unwrap_or(0u32) != 1 {
        return None
    }

    if let Ok(proxy_server) = internet_settings.get_value("ProxyServer") {
        let proxy_server: String = proxy_server;
        let proxy_list = parse_proxy_list(&proxy_server);

        if proxy_list.is_empty() {
            return None
        }

        let mut proxy_config: ProxyConfig = Default::default();
        proxy_config.proxies.extend(proxy_list);

        if let Ok(proxy_override) = internet_settings.get_value("ProxyOverride") {
            let proxy_override: String = proxy_override;
            let bypass_list = parse_bypass_list(&proxy_override);
            proxy_config.whitelist.extend(bypass_list);
        }

        if whitelist.contains("<local>") {
            proxy_config.exclude_simple = true;
        }

        return Some(proxy_config)
    }

    None
}

fn win_inet_get_current_user_config() -> Option<ProxyConfig> {
    if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(REG_CONNECTIONS) {
        if win_inet_get_autoconfig_type(key) != AutoconfigType::None {
            return None
        }
    }

    if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(REG_SETTINGS) {
        return win_inet_get_proxy_config(key)
    }

    None
}

fn win_inet_get_local_machine_config() -> Option<ProxyConfig> {
    if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(REG_CONNECTIONS) {
        if win_inet_get_autoconfig_type(key) != AutoconfigType::None {
            return None
        }
    }

    if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(REG_SETTINGS) {
        return win_inet_get_proxy_config(key)
    }

    None
}

fn win_http_get_default_config() -> Option<ProxyConfig> {
    let mut proxy_info: WINHTTP_PROXY_INFO;

    let result = unsafe {
        proxy_info = std::mem::zeroed();
        WinHttpGetDefaultProxyConfiguration(&mut proxy_info)
     };

    if result == FALSE || proxy_info.dwAccessType != WINHTTP_ACCESS_TYPE_NAMED_PROXY {
        return None
    }

    let proxy_server = unsafe { lpwstr_null_to_string(proxy_info.lpszProxy) };
    let proxy_list = parse_proxy_list(&proxy_server.unwrap_or_default());

    if proxy_list.is_empty() {
        return None
    }

    let mut proxy_config: ProxyConfig = Default::default();
    proxy_config.proxies.extend(proxy_list);

    let proxy_bypass = unsafe { lpwstr_null_to_string(proxy_info.lpszProxyBypass) };

    if let Some(proxy_bypass) = proxy_bypass {
        let bypass_list = parse_bypass_list(&proxy_bypass);
        proxy_config.whitelist.extend(bypass_list);
    }

    if whitelist.contains("<local>") {
        proxy_config.exclude_simple = true;
    }
    
    Some(proxy_config)
}

#[test]
fn parse_exceptions_test() {
    let bypass_list = "  <local>;.microsoft.com  ;  192.168.*.* 172.16.10.*";
    let parsed = parse_bypass_list(bypass_list);
    assert_eq!(parsed, vec!["<local>", ".microsoft.com", "192.168.*.*", "172.16.10.*"])
}

#[test]
fn parse_proxies_test() {
    let hm = parse_proxy_list("http=1.2.3.4:80");
    assert_eq!(1, hm.len());
    assert_eq!("1.2.3.4:80", hm.get("http").unwrap());

    let hm = parse_proxy_list("1.2.3.4;https=http://8.8.8.8");
    assert_eq!(2, hm.len());
    assert_eq!("1.2.3.4", hm.get("http").unwrap());
    assert_eq!("http://8.8.8.8", hm.get("https").unwrap());

    let hm = parse_proxy_list("http://1.2.3.4;https=8.8.8.8   http=9.8.7.6:123");
    assert_eq!(2, hm.len());
    assert_eq!("9.8.7.6:123", hm.get("http").unwrap());
}

pub(crate) fn get_proxy_config() -> Result<ProxyConfig> {
    let win_inet_user_proxy = win_inet_get_current_user_config();

    if !win_inet_is_per_user() || win_inet_user_proxy.is_none() {
        if let Some(proxy_config) = win_inet_get_local_machine_config() {
            return Ok(proxy_config)
        }
    }

    if let Some(proxy_config) = win_inet_user_proxy {
        return Ok(proxy_config)
    }

    win_http_get_default_config().ok_or(ProxyConfigError::NoProxyConfiguredError)
}
