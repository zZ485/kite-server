use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::CONFIG;
use crate::error::Result;

pub fn encode_jwt<T: Serialize>(claims: &T) -> Result<String> {
    let key = &CONFIG.server.secret.as_ref();
    let encoding_key = jsonwebtoken::EncodingKey::from_secret(key);

    Ok(jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &encoding_key,
    )?)
}

pub fn decode_jwt<T: DeserializeOwned>(token: &str) -> Option<T> {
    let key = &CONFIG.server.secret.as_ref();
    let decoding_key = jsonwebtoken::DecodingKey::from_secret(key);
    let option = jsonwebtoken::Validation {
        validate_exp: false,
        ..jsonwebtoken::Validation::default()
    };
    let t = jsonwebtoken::decode::<T>(token, &decoding_key, &option);

    if let Ok(token_data) = t {
        Some(token_data.claims)
    } else {
        None
    }
}

pub fn validate_jwt<T: DeserializeOwned>(token: &str) -> bool {
    decode_jwt::<T>(token).is_some()
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_normal_parse_auth_line() {
//         let line = HeaderValue::from_static("Basic YWxhZGRpbjpvcGVuc2VzYW1l");
//         let (auth_type, auth_credential) = parse_auth_line(&line)
//             .expect("Failed to parse: \"Basic YWxhZGRpbjpvcGVuc2VzYW1l\"");
//
//         assert_eq!(auth_type, "Basic");
//         assert_eq!(auth_credential, "YWxhZGRpbjpvcGVuc2VzYW1l");
//     }
//
//     #[test]
//     fn test_bad_parse_auth_line() {
//         let line = HeaderValue::from_static("Basic");
//         let result = parse_auth_line(&line);
//         assert_eq!(result, None);
//
//         let line = HeaderValue::from_static("");
//         let result = parse_auth_line(&line);
//         assert_eq!(result, None);
//
//         let line = HeaderValue::from_static("Basic p1 p2");
//         let result = parse_auth_line(&line);
//         assert_eq!(result, None);
//     }
//
//     #[test]
//     fn test_normal_jwt_decode() {
//         let key = "secret";
//         let jwt_string = r"eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1aWQiOjEwfQ.jNHERe-nmbsUSi4mn3z9IsLTuN5dQGdHHlgFRh5mNUA";
//         let claims = decode_jwt(jwt_string, key).unwrap();
//         assert_eq!(claims, JwtClaims {
//             uid: 10
//         });
//     }
// }
