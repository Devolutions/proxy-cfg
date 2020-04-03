use std::ptr;

use core_foundation::{ base::*, array::*, dictionary::*, string::*, number::* };
use system_configuration_sys::dynamic_store_copy_specific;

use super::*;

fn get_array_value(dictionary: &CFDictionary<CFString, CFType>, key: &'static str) -> Option<CFArray> {
    let key = CFString::from_static_string(key);
    dictionary.find(key)
        .and_then(|v| v.downcast::<CFArray>())
}

fn get_string_value(dictionary: &CFDictionary<CFString, CFType>, key: &'static str) -> Option<String> {
    let key = CFString::from_static_string(key);
    dictionary.find(key)
        .and_then(|v| v.downcast::<CFString>()) 
        .map(|v| v.to_string())
}

fn get_i32_value(dictionary: &CFDictionary<CFString, CFType>, key: &'static str) -> Option<i32> {
    let key = CFString::from_static_string(key);
    dictionary.find(key)
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|v| v.to_i32())
}

pub(crate) fn get_proxy_config() -> Result<ProxyConfig> {
    let proxies_ref = unsafe {
        dynamic_store_copy_specific::SCDynamicStoreCopyProxies(ptr::null())
    };

    let mut proxy_config: ProxyConfig = Default::default();

    if proxies_ref.is_null() {
        return Ok(proxy_config)
    }

    let proxies: CFDictionary<CFString, CFType> = unsafe { 
        CFDictionary::wrap_under_create_rule(proxies_ref) 
    };

    if get_i32_value(&proxies, "HTTPEnable").unwrap_or(0) == 1 {
        let mut url = get_string_value(&proxies, "HTTPProxy").unwrap_or_default();
        if let Some(port) = get_i32_value(&proxies, "HTTPPort") {
            url = format!("{}:{}", url, port);
        } 

        proxy_config.proxies.insert("http".into(), url);
    }

    if get_i32_value(&proxies, "HTTPSEnable").unwrap_or(0) == 1 {
        let mut url = get_string_value(&proxies, "HTTPSProxy").unwrap_or_default();
        if let Some(port) = get_i32_value(&proxies, "HTTPSPort") {
            url = format!("{}:{}", url, port);
        } 

        proxy_config.proxies.insert("https".into(), url);
    }

    if get_i32_value(&proxies, "FTPEnable").unwrap_or(0) == 1 {
        let mut url = get_string_value(&proxies, "FTPProxy").unwrap_or_default();
        if let Some(port) = get_i32_value(&proxies, "FTPPort") {
            url = format!("{}:{}", url, port);
        } 

        proxy_config.proxies.insert("ftp".into(), url);
        // TODO kSCPropNetProxiesFTPPassive
    }

    if get_i32_value(&proxies, "ExcludeSimpleHostnames").unwrap_or(0) == 1 {
        proxy_config.exclude_simple = true;
    }

    if let Some(exceptions_list) = get_array_value(&proxies, "ExceptionsList") {
        let cf_strings = exceptions_list.iter().map(|ptr| {
            unsafe { CFString::wrap_under_get_rule(CFStringRef::from_void_ptr(*ptr)) }
        }).collect::<Vec<_>>();

        proxy_config.whitelist.extend(cf_strings.iter().map(|s| s.to_string().to_lowercase()));
    }

    Ok(proxy_config)
}

#[test]
fn get_proxy_config_test() {
    let _ = get_proxy_config();
}