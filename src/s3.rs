use std::time::{SystemTime, UNIX_EPOCH};
use futures::Future;
use rusoto_core::{ProvideAwsCredentials, Region};
use rusoto_core::credential::{AwsCredentials, EnvironmentProvider};
use rusoto_s3::PutObjectRequest;
use rusoto_s3::util::PreSignedRequest;

pub struct S3Access {
    bucket: String,
    region: Region,
    creds: AwsCredentials,
}

impl S3Access {
    pub fn new(bucket: String) -> S3Access {
        let region = Region::default(); // reads from environment var
        let creds = EnvironmentProvider.credentials().wait()
            .expect("couldn't build AWS credentials");
        S3Access { bucket, region, creds }
    }
}

#[derive(Serialize, Deserialize)]
pub struct UploadRequest {
    filename: String,
    file_type: String
}

#[derive(Serialize, Deserialize)]
pub struct UploadResponse {
    url: String
}

fn expiration_epoch(timer: u64) -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
    since_the_epoch.as_secs() + timer
}

pub fn sign_upload(s3: &S3Access, _directory: &str, req: UploadRequest) -> UploadResponse {
    let put_req = PutObjectRequest {
        bucket: s3.bucket.clone(),
        key: req.filename.clone(), // TODO: use the directory here
        content_type: Some(req.file_type.clone()),
        // TODO: expires?
        acl: Some(String::from("public-read")),
        ..Default::default()
    };
    let url = put_req.get_presigned_url(&s3.region, &s3.creds);
    UploadResponse { url: url }
}

#[cfg(test)]
mod test {
    use super::*;
    use rusoto_core::credential::StaticProvider;
    use reqwest::{StatusCode, Client};

    #[test]
    fn upload_unauthorized() {
        let creds = StaticProvider::new_minimal(String::from("foo"), String::from("baz"))
            .credentials().wait().expect("couldn't make static creds");
        let bucket = String::from("photothing-heroku-dev");
        let access = S3Access { bucket, creds, region: Region::UsEast1 };
        let req = UploadRequest {
            filename: String::from(""), file_type: String::from("")
        };

        let url = sign_upload(&access, "automation", req).url;
        assert!(url.starts_with("https://"));

        let client = Client::new();
        let res = client.put(&url)
            .body("some content")
            .send()
            .expect("request failed");
        assert_eq!(res.status(), StatusCode::Forbidden);
    }

    #[test]
    fn upload_integration_test() {
        let creds = EnvironmentProvider.credentials().wait()
            .expect("couldn't build AWS credentials");
        let bucket = String::from("photothing-heroku-dev");
        let access = S3Access { bucket, creds, region: Region::UsEast1 };
        let req = UploadRequest {
            filename: String::from("upload.txt"), file_type: String::from("text/plain")
        };

        let url = sign_upload(&access, "automation", req).url;
        assert!(url.starts_with("https://"));

        let client = Client::new();
        let res = client.put(&url)
            .body("foobizbaz")
            .send()
            .expect("request failed");
        assert_eq!(res.status(), StatusCode::Ok);
    }
}
