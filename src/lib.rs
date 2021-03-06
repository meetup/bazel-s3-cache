// todo: s3 bucket lifecycle management
//
#[macro_use]
extern crate cpython;
#[macro_use]
extern crate lando;
extern crate rusoto_core;
extern crate rusoto_s3;
#[macro_use]
extern crate serde_derive;
extern crate base64;
extern crate envy;
extern crate futures;
extern crate http;
#[cfg(test)]
extern crate url;

// Std lib

use std::time::Duration;

// Third party
use futures::future::Future;
use http::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, LOCATION};
use http::{Method, StatusCode};
use lando::Response;
use rusoto_core::credential::{AwsCredentials, ChainProvider, ProvideAwsCredentials};
use rusoto_s3::util::PreSignedRequest;
use rusoto_s3::{GetObjectRequest, HeadObjectRequest, PutObjectRequest, S3, S3Client};

const AUTH_PREFIX: &[u8] = b"Basic ";

#[derive(Deserialize, Default)]
struct Config {
    bucket: String,
    username: String,
    password: String,
}

fn credentials() -> ChainProvider {
    let mut chain = ChainProvider::new();
    chain.set_timeout(Duration::from_millis(200));
    chain
}

fn get(bucket: String, key: String, credentials: &AwsCredentials) -> String {
    GetObjectRequest {
        bucket,
        key,
        ..Default::default()
    }.get_presigned_url(&Default::default(), &credentials, &Default::default())
}

fn put(bucket: String, key: String, credentials: &AwsCredentials) -> String {
    PutObjectRequest {
        bucket,
        key,
        ..Default::default()
    }.get_presigned_url(&Default::default(), &credentials, &Default::default())
}

fn exists<C>(client: C, bucket: String, key: String) -> bool
where
    C: S3,
{
    client
        .head_object(HeadObjectRequest {
            bucket,
            key,
            ..Default::default()
        })
        .sync()
        .is_ok()
}

/// Return true if provided authz header matches config
fn authenticated(config: &Config, authz: &[u8]) -> bool {
    if !authz.starts_with(&AUTH_PREFIX) {
        return false;
    }
    base64::decode(&authz[AUTH_PREFIX.len()..])
        .ok()
        .into_iter()
        .filter_map(|bytes| String::from_utf8(bytes).ok())
        .any(|decoded| {
            let Config {
                username, password, ..
            } = config;
            match &decoded.splitn(2, ':').collect::<Vec<_>>()[..] {
                [user, pass] => user == username && pass == password,
                _ => false,
            }
        })
}

fn key(path: &str) -> &str {
    if path.starts_with('/') {
        &path[1..]
    } else {
        path
    }
}

gateway!(|request, _| {
    let config = envy::from_env::<Config>()?;
    println!(
        "recv {} {} {:?} {:?}",
        request.method(),
        request.uri().path(),
        request.headers().get(CONTENT_TYPE),
        request.headers().get(CONTENT_LENGTH)
    );
    if request
        .headers()
        .get(AUTHORIZATION)
        .into_iter()
        .any(|authz| authenticated(&config, authz.as_bytes()))
    {
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(())?);
    }

    match request.method() {
        &Method::GET | &Method::PUT => Ok(Response::builder()
            .status(StatusCode::MOVED_PERMANENTLY)
            .header(
                LOCATION,
                match request.method() {
                    &Method::GET => get(
                        config.bucket,
                        key(request.uri().path()).into(),
                        &credentials().credentials().wait()?,
                    ),
                    _ => put(
                        config.bucket,
                        key(request.uri().path()).into(),
                        &credentials().credentials().wait()?,
                    ),
                },
            )
            .body(())?),
        &Method::HEAD => {
            let status = if exists(
                S3Client::new(Default::default()),
                config.bucket,
                key(request.uri().path()).into(),
            ) {
                StatusCode::OK
            } else {
                StatusCode::NOT_FOUND
            };
            Ok(Response::builder().status(status).body(())?)
        }
        _ => Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(())?),
    }
});

#[cfg(test)]
mod tests {

    use http::Uri;
    use rusoto_core::credential::AwsCredentials;
    use url::form_urlencoded;

    use super::{authenticated, get, key, put, Config};

    #[test]
    fn key_strips_prefix_slash() {
        assert_eq!(key("/foo/bar"), "foo/bar")
    }

    #[test]
    fn key_left_as_is_without_prefix_slash() {
        assert_eq!(key("foo/bar"), "foo/bar")
    }

    #[test]
    fn get_link() {
        let link: Uri = get(
            "foo".into(),
            "bar/car".into(),
            &AwsCredentials::new("boom", "zoom", Default::default(), Default::default()),
        ).parse()
            .unwrap();
        assert_eq!(Some("s3.amazonaws.com"), link.host());
        assert_eq!("/foo/bar/car", link.path());
        assert!(
            form_urlencoded::parse(link.query().unwrap().as_bytes())
                .into_iter()
                .any(|(k, _)| k.starts_with("X-Amz-"))
        );
    }

    #[test]
    fn put_link() {
        let link: Uri = put(
            "foo".into(),
            "bar/car".into(),
            &AwsCredentials::new("boom", "zoom", Default::default(), Default::default()),
        ).parse()
            .unwrap();
        assert_eq!(Some("s3.amazonaws.com"), link.host());
        assert_eq!("/foo/bar/car", link.path());
        assert!(
            form_urlencoded::parse(link.query().unwrap().as_bytes())
                .into_iter()
                .any(|(k, _)| k.starts_with("X-Amz-"))
        );
    }

    #[test]
    fn authenticated_rejects_invalid_requests() {
        assert!(!authenticated(
            &Config {
                username: "foo".into(),
                password: "bar".into(),
                ..Default::default()
            },
            "test".as_bytes()
        ))
    }

    #[test]
    fn authenticated_rejects_partially_valid_requests() {
        assert!(!authenticated(
            &Config {
                username: "foo".into(),
                password: "bar".into(),
                ..Default::default()
            },
            "Basic Zm9v".as_bytes()
        ))
    }

    #[test]
    fn authenticated_permits_valid_requests_with_passwords_containing_colon() {
        assert!(authenticated(
            &Config {
                username: "foo".into(),
                password: "bar:baz".into(),
                ..Default::default()
            },
            "Basic Zm9vOmJhcjpiYXo=".as_bytes()
        ))
    }

    #[test]
    fn authenticated_permits_valid_requests() {
        assert!(authenticated(
            &Config {
                username: "foo".into(),
                password: "bar".into(),
                ..Default::default()
            },
            "Basic Zm9vOmJhcg==".as_bytes()
        ))
    }
}
