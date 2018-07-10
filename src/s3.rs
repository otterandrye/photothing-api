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
    url: String,
    directory: String,
    filename: String,
    get_url: String,
}

pub fn sign_upload(s3: &S3Access, directory: &str, req: UploadRequest) -> UploadResponse {
    let put_req = PutObjectRequest {
        bucket: s3.bucket.clone(),
        key: format!("{}/{}", directory, req.filename.clone()),
        content_type: Some(req.file_type.clone()),
        ..Default::default()
    };
    let url = put_req.get_presigned_url(&s3.region, &s3.creds);

    // TODO: point this at the CDN rather than the dev s3 bucket
    let get_url = format!("http://{}.s3-website-{}.amazonaws.com/{}/{}", s3.bucket, s3.region.name(), directory, req.filename);
    UploadResponse {
        url,
        get_url,
        directory: directory.to_owned(),
        filename: req.filename.clone()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand;
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
        assert_eq!(res.status(), StatusCode::Forbidden, "unathorized request didn't fail");
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

        let response = sign_upload(&access, "automation", req);
        let url = response.url;
        assert!(url.starts_with("https://"));
        assert_eq!(response.filename, "upload.txt");
        assert_eq!(response.directory, "automation");

        // make sure we can upload content to the returned presigned url
        let content: i64 = rand::random();
        let body = format!("foobizbaz={}", content);
        let client = Client::new();
        let res = client.put(&url)
            .body(body.clone())
            .send()
            .expect("request failed");
        assert_eq!(res.status(), StatusCode::Ok, "upload request got 200 status");

        // and that we can fetch it back using the get_url
        let mut get = client.get(&response.get_url).send().expect("request failed");
        assert_eq!(get.status(), StatusCode::Ok, "got uploaded content");
        assert_eq!(get.text().expect("no text"), body, "got correct content back");
    }
}
