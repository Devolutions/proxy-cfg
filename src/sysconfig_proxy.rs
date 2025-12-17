//! This module reads the proxy configuration file /etc/sysconfig/proxy which
//! exists on Red Hat Enterprise Linux and related Linux systems. For a
//! description of the configuration file format see:
//! <https://www.novell.com/support/kb/doc.php?id=7006845>
//! <https://www.suse.com/de-de/support/kb/doc/?id=7006845>

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::{Error, ProxyConfig, Result};

/// Extract proxy information from /etc/sysconfig/proxy if the file is available
/// and formatted correctly.
pub(crate) fn get_proxy_config() -> Result<Option<ProxyConfig>> {
    get_proxy_config_from_file("/etc/sysconfig/proxy")
}

/// The same as `get_proxy_config()` but this function expects a file's path as an
/// argument.
fn get_proxy_config_from_file<P: AsRef<Path>>(config_file: P) -> Result<Option<ProxyConfig>> {
    let mut proxy_config: ProxyConfig = Default::default();
    if !config_file.as_ref().exists() {
        return Ok(None);
    }
    let map = read_key_value_pairs_from_file(config_file)?;
    if let Some(enabled) = map.get("PROXY_ENABLED") {
        match enabled.as_str() {
            "yes" => (), // Continue running this function.
            "no" => return Ok(None),
            _ => return Err(Error::InvalidConfig), // Consider all other values as illegal.
        }
    } else {
        return Err(Error::InvalidConfig); // Missing PROXY_ENABLED directive.
    }

    // Determine the proxies.
    let schemes = ["HTTP", "HTTPS", "FTP"];
    for scheme in schemes.iter() {
        let key = format!("{}_PROXY", scheme);
        if let Some(proxy) = map.get(&key) {
            // Check if ${SCHEME}_PROXY is defined.
            let scheme = scheme.to_lowercase();
            proxy_config.proxies.insert(scheme, proxy.clone());
        }
    }

    // Determine the list of domains that should not be requested through the proxy.
    if let Some(no_proxy) = map.get("NO_PROXY") {
        for no_proxy_url in no_proxy.split(",") {
            proxy_config
                .whitelist
                .insert(no_proxy_url.trim().to_owned().to_lowercase());
        }
    }

    Ok(Some(proxy_config))
}

/// Read a file which contains key-value pairs that are separated by an equals
/// sign and each value has to be surrounded by double quotes.. Each key-value
/// pair has to be on it's own line. Example:
///
/// ```plain
/// foo="42"
/// bar="43"
///
/// baz="44"
/// quux="foobar"
/// ```
///
/// The file may contain empty lines. Values in double quotes will be converted
/// into values without the outer double quotes.
///
/// Note that the current implementation does not trim whitespace and the ends
/// of a line. It is currently assumed that leading or trailing whitespace is
/// not part of the file format.
///
fn read_key_value_pairs_from_file<P: AsRef<Path>>(file: P) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();
    let file = File::open(file)?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?; // Get rid of IO errors.

        if line.is_empty() {
            continue; // Skip empty line.
        }

        if let Some(pos) = line.find("=\"") {
            let key = line[0..pos].to_string();
            let value = strip_after_quote(&line[pos + 2..]).to_owned();
            result.insert(key, value);
        } else {
            // There has to be an equals sign in this file.
            return Err(Error::InvalidConfig);
        }
    }

    Ok(result)
}

/// Remove trailing double quote (and anything thereafter) from a string.
fn strip_after_quote(s: &str) -> &str {
    match s.find('"') {
        // We will tolerate more trailing characters after a quote.
        Some(pos) => &s[..pos],
        None => s, // Be generous and assume that there should have been a double quote at the very end.
    }
}

#[cfg(test)]
mod tests {
    extern crate tempfile;

    use std::io::Write;

    use self::tempfile::NamedTempFile;
    use super::{get_proxy_config_from_file, read_key_value_pairs_from_file, strip_after_quote};

    /// Write a string to a temporary file.
    fn spit(contents: &str) -> NamedTempFile {
        let mut outfile = NamedTempFile::new().expect("failed to create temporary file");
        let _ = outfile.write(contents.as_bytes());
        outfile
    }

    #[test]
    fn test_read_key_value_pairs_from_file() {
        let file = spit(
            r##"
foo="bar"
baz="quux"

spam="eggs"

"##,
        );
        let map = read_key_value_pairs_from_file(file.path()).unwrap();
        assert!(&map["foo"] == "bar");
        assert!(&map["baz"] == "quux");
        assert!(&map["spam"] == "eggs");

        let file = spit(
            r##"
foo="bar"
baz "quux"

spam="eggs"

"##,
        );
        assert!(read_key_value_pairs_from_file(file.path()).is_err());
    }

    #[test]
    fn test_get_proxy_config() {
        let file = spit(
            r##"HTTP_PROXY="http://1.2.3.4"
HTTPS_PROXY="https://1.2.3.4:8000""##,
        );
        assert!(get_proxy_config_from_file(file.path()).is_err()); // Missing PROXY_ENABLED.

        let file = spit(
            r##"HTTP_PROXY="http://1.2.3.4"
HTTPS_PROXY="https://1.2.3.4:8000"
PROXY_ENABLED="no""##,
        );
        assert!(get_proxy_config_from_file(file.path()).is_ok());

        let file = spit(
            r##"HTTP_PROXY="http://1.2.3.4"
HTTPS_PROXY="https://1.2.3.4:8000"
PROXY_ENABLED="yes""##,
        );
        let config = get_proxy_config_from_file(file.path()).unwrap().unwrap();
        assert_eq!(&config.proxies["http"], "http://1.2.3.4");
        assert_eq!(&config.proxies["https"], "https://1.2.3.4:8000");
    }

    #[test]
    fn test_whitelist() {
        // It would be nicer to test this directly with get_proxy_for_url(), but
        // then we would need to overwrite /etc/sysconfig/proxy, which is
        // something a unit test should not do.

        let file = spit(
            r##"HTTP_PROXY="http://1.2.3.4"
HTTPS_PROXY="https://1.2.3.4:8000"
NO_PROXY="localhost,1.2.3.4,5.6.7.8"
PROXY_ENABLED="yes""##,
        );
        let config = get_proxy_config_from_file(file.path()).unwrap().unwrap();
        for no_proxy in config.whitelist {
            match no_proxy.as_str() {
                "localhost" => (),
                "1.2.3.4" => (),
                "5.6.7.8" => (),
                _ => panic!("Expecting no proxy element to be one of \"localhost\", \"1.2.3.4\", or \"5.6.7.8\""),
            }
        }
    }

    #[test]
    fn test_unquote() {
        assert_eq!(strip_after_quote("foo"), "foo");
        assert_eq!(strip_after_quote("\"foo\""), "");
        assert_eq!(strip_after_quote("\"foo bar"), "");
        assert_eq!(strip_after_quote("foo\"bar"), "foo");
    }

    #[test]
    fn test_with_example_from_specification() {
        let file = spit(
            r##"
PROXY_ENABLED="yes"

HTTP_PROXY="http://192.168.0.1"
HTTPS_PROXY="http://192.168.0.1"
FTP_PROXY="http://192.168.0.1"
NO_PROXY="localhost, 127.0.0.1"
"##,
        );
        let config = get_proxy_config_from_file(file.path()).unwrap().unwrap();
        assert_eq!(&config.proxies["http"], "http://192.168.0.1");
        assert_eq!(&config.proxies["https"], "http://192.168.0.1");
        assert_eq!(&config.proxies["ftp"], "http://192.168.0.1");
        assert!(config.whitelist.contains("localhost"));
        assert!(config.whitelist.contains("127.0.0.1"));
    }

    #[test]
    fn test_file_without_quoting() {
        let file = spit(
            r##"PROXY_ENABLED="yes"
HTTP_PROXY=http://localhost"##,
        );
        match get_proxy_config_from_file(file.path()) {
            Err(_) => (), // All is fine.
            _ => panic!("Expected error when parsing file without proper quoting"),
        }
    }
}
