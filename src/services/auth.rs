use actix_web::dev::{Payload, PayloadStream};
use actix_web::error::{Error, ErrorUnauthorized};
use actix_web::{FromRequest, HttpRequest};
use futures::future::{err, ok, Ready};

use crate::jwt::decode_jwt;
use crate::services::JwtToken;

use super::get_auth_bearer_value;

impl FromRequest for JwtToken {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload<PayloadStream>) -> Self::Future {
        // Get authentication header.
        if let Some(auth_string) = req.headers().get("Authorization") {
            // If authentication type is "Bearer"
            if let Some(jwt_string) = get_auth_bearer_value(auth_string) {
                // Unpack JWT to verify credential
                if let Some(token) = decode_jwt::<JwtToken>(jwt_string) {
                    return ok(token);
                }
            }
        }
        err(ErrorUnauthorized("Unauthorized"))
    }
}

// TODO: Implement ServiceRequest for JwtToken
