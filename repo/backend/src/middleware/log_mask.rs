use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Request, Response};
use serde_json::Value;
use std::io::Cursor;

/// A Rocket fairing that masks sensitive fields in JSON response bodies.
pub struct LogMaskFairing;

/// Fields whose values will be replaced with `"***MASKED***"` in JSON
/// response bodies.
const SENSITIVE_FIELDS: &[&str] = &[
    "password",
    "password_hash",
    "voucher_code",
    "token",
    "session_id",
    "session_cookie",
    "cookie",
];

#[rocket::async_trait]
impl Fairing for LogMaskFairing {
    fn info(&self) -> Info {
        Info {
            name: "Log Mask Fairing",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        let content_type = response
            .headers()
            .get_one("Content-Type")
            .unwrap_or_default();

        if !content_type.contains("application/json") {
            return;
        }

        // Read the body, mask it, then put it back.
        if let Ok(body_str) = response.body_mut().to_string().await {
            if let Ok(mut json) = serde_json::from_str::<Value>(&body_str) {
                mask_json_fields(&mut json, SENSITIVE_FIELDS);
                let masked = serde_json::to_string(&json).unwrap_or(body_str.clone());

                // Log the masked version
                tracing::debug!(body = %masked, "response body (masked)");

                // Put the ORIGINAL (unmasked) body back so the client gets real data.
                response.set_sized_body(body_str.len(), Cursor::new(body_str));
            }
        }
    }
}

/// Recursively walk a JSON value and replace values of keys in `fields` with
/// `"***MASKED***"`.
pub fn mask_json_fields(value: &mut Value, fields: &[&str]) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if fields.iter().any(|f| f.eq_ignore_ascii_case(key)) {
                    *val = Value::String("***MASKED***".into());
                } else {
                    mask_json_fields(val, fields);
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                mask_json_fields(item, fields);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn masked_string(v: &Value) -> Option<&str> {
        v.as_str().filter(|s| *s == "***MASKED***")
    }

    #[test]
    fn masks_password_field() {
        let mut v = json!({"password": "secret123"});
        mask_json_fields(&mut v, SENSITIVE_FIELDS);
        assert!(masked_string(&v["password"]).is_some());
    }

    #[test]
    fn masks_session_cookie_field() {
        let mut v = json!({"session_cookie": "abc.def"});
        mask_json_fields(&mut v, SENSITIVE_FIELDS);
        assert!(masked_string(&v["session_cookie"]).is_some());
    }

    #[test]
    fn masks_voucher_code_field() {
        let mut v = json!({"voucher_code": "VC-12345"});
        mask_json_fields(&mut v, SENSITIVE_FIELDS);
        assert!(masked_string(&v["voucher_code"]).is_some());
    }

    #[test]
    fn non_sensitive_fields_are_untouched() {
        let mut v = json!({"username": "alice", "email": "alice@example.com"});
        mask_json_fields(&mut v, SENSITIVE_FIELDS);
        assert_eq!(v["username"], "alice");
        assert_eq!(v["email"], "alice@example.com");
    }

    #[test]
    fn masks_fields_case_insensitively() {
        let mut v = json!({"Password": "s3cr3t!", "SESSION_COOKIE": "tok"});
        mask_json_fields(&mut v, SENSITIVE_FIELDS);
        assert!(masked_string(&v["Password"]).is_some());
        assert!(masked_string(&v["SESSION_COOKIE"]).is_some());
    }

    #[test]
    fn masks_nested_sensitive_fields() {
        let mut v = json!({"data": {"user": {"password_hash": "bcrypt_hash", "name": "bob"}}});
        mask_json_fields(&mut v, SENSITIVE_FIELDS);
        assert!(masked_string(&v["data"]["user"]["password_hash"]).is_some());
        assert_eq!(v["data"]["user"]["name"], "bob");
    }

    #[test]
    fn masks_fields_inside_arrays() {
        let mut v = json!([{"session_cookie": "tok1"}, {"session_cookie": "tok2"}]);
        mask_json_fields(&mut v, SENSITIVE_FIELDS);
        assert!(masked_string(&v[0]["session_cookie"]).is_some());
        assert!(masked_string(&v[1]["session_cookie"]).is_some());
    }

    #[test]
    fn login_response_shape_has_session_cookie_masked() {
        // Mirrors the actual LoginResponse DTO: {"success":true,"data":{"session_cookie":...,"user":{...}}}
        let mut v = json!({
            "success": true,
            "data": {
                "session_cookie": "signed.value",
                "user": {"id": 1, "username": "alice", "roles": ["Customer"]}
            },
            "error": null
        });
        mask_json_fields(&mut v, SENSITIVE_FIELDS);
        assert!(masked_string(&v["data"]["session_cookie"]).is_some());
        // User fields not masked
        assert_eq!(v["data"]["user"]["username"], "alice");
    }

    // ── extra coverage ─────────────────────────────────────────────────────

    #[test]
    fn masks_token_and_session_id_fields() {
        let mut v = json!({"token": "abc123", "session_id": "sid-42"});
        mask_json_fields(&mut v, SENSITIVE_FIELDS);
        assert!(masked_string(&v["token"]).is_some());
        assert!(masked_string(&v["session_id"]).is_some());
    }

    #[test]
    fn fairing_info_reports_response_kind() {
        let f = LogMaskFairing;
        let info = f.info();
        assert_eq!(info.name, "Log Mask Fairing");
        // Kind is a bitflag-style set in Rocket; verify Response is present.
        assert!(
            (info.kind & rocket::fairing::Kind::Response) == rocket::fairing::Kind::Response,
            "fairing must declare Response kind"
        );
    }

    #[test]
    fn empty_object_is_unchanged() {
        let mut v = json!({});
        let before = v.clone();
        mask_json_fields(&mut v, SENSITIVE_FIELDS);
        assert_eq!(v, before);
    }

    #[test]
    fn deeply_nested_list_of_lists_is_masked() {
        let mut v = json!({
            "batches": [
                {"items": [{"password": "p1"}, {"password": "p2"}]},
                {"items": [{"token": "t"}]}
            ]
        });
        mask_json_fields(&mut v, SENSITIVE_FIELDS);
        assert!(masked_string(&v["batches"][0]["items"][0]["password"]).is_some());
        assert!(masked_string(&v["batches"][0]["items"][1]["password"]).is_some());
        assert!(masked_string(&v["batches"][1]["items"][0]["token"]).is_some());
    }

    #[test]
    fn non_string_sensitive_values_are_replaced_with_string() {
        // Even if a password lands as a number in some bogus payload, it is
        // still replaced with a masked placeholder.
        let mut v = json!({"password": 12345});
        mask_json_fields(&mut v, SENSITIVE_FIELDS);
        assert!(masked_string(&v["password"]).is_some());
    }

    #[test]
    fn empty_sensitive_list_leaves_body_untouched() {
        let mut v = json!({"password": "p", "other": "x"});
        let before = v.clone();
        mask_json_fields(&mut v, &[]);
        assert_eq!(v, before);
    }
}
