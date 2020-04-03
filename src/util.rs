
// use super::*;

// pub fn parse_addr_default_scheme(scheme: &str, addr: &str) -> Result<Url> {
//     let split: Vec<&str> = addr.split("://").collect();
//     if split.len() == 2 {
//         Ok(Url::parse(addr)?)
//     } else if split.len() == 1 {
//         Ok(Url::parse(&format!("{}://{}", scheme, addr))?)
//     } else {
//         Err(InvalidConfigError)
//     }
// }
