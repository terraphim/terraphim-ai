// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use bytes::Bytes;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Formatter;

use async_trait::async_trait;
use http::header::CONTENT_DISPOSITION;
use http::header::CONTENT_TYPE;
use http::Request;
use serde::Deserialize;
use serde::Serialize;

use atomic_lib::agents::Agent;
use atomic_lib::client::get_authentication_headers;
use atomic_lib::commit::sign_message;

use crate::raw::adapters::kv;
use crate::raw::new_json_deserialize_error;
use crate::raw::new_json_serialize_error;
use crate::raw::new_request_build_error;
use crate::raw::normalize_path;
use crate::raw::normalize_root;
use crate::raw::percent_encode_path;
use crate::raw::AsyncBody;
use crate::raw::FormDataPart;
use crate::raw::HttpClient;
use crate::raw::Multipart;
use crate::Builder;
use crate::Scheme;
use crate::*;

/// Atomicserver service support.
#[doc = include_str!("docs.md")]
#[derive(Default)]
pub struct AtomicserverBuilder {
    root: Option<String>,
    endpoint: Option<String>,
    private_key: Option<String>,
    public_key: Option<String>,
    parent_resource_id: Option<String>,
}

impl AtomicserverBuilder {
    /// Set the root for Atomicserver.
    pub fn root(&mut self, path: &str) -> &mut Self {
        self.root = Some(path.into());
        self
    }

    /// Set the server address for Atomicserver.
    pub fn endpoint(&mut self, endpoint: &str) -> &mut Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set the private key for agent used for Atomicserver.
    pub fn private_key(&mut self, private_key: &str) -> &mut Self {
        self.private_key = Some(private_key.into());
        self
    }

    /// Set the public key for agent used for Atomicserver.
    /// For example, if the subject URL for the agent being used
    /// is ${endpoint}/agents/lTB+W3C/2YfDu9IAVleEy34uCmb56iXXuzWCKBVwdRI=
    /// Then the required public key is `lTB+W3C/2YfDu9IAVleEy34uCmb56iXXuzWCKBVwdRI=`
    pub fn public_key(&mut self, public_key: &str) -> &mut Self {
        self.public_key = Some(public_key.into());
        self
    }

    /// Set the parent resource id (url) that Atomicserver uses to store resources under.
    pub fn parent_resource_id(&mut self, parent_resource_id: &str) -> &mut Self {
        self.parent_resource_id = Some(parent_resource_id.into());
        self
    }
}

impl Builder for AtomicserverBuilder {
    const SCHEME: Scheme = Scheme::Atomicserver;
    type Accessor = AtomicserverBackend;

    fn from_map(map: HashMap<String, String>) -> Self {
        let mut builder = AtomicserverBuilder::default();

        map.get("root").map(|v| builder.root(v));
        map.get("endpoint").map(|v| builder.endpoint(v));
        map.get("private_key").map(|v| builder.private_key(v));
        map.get("public_key").map(|v| builder.public_key(v));
        map.get("parent_resource_id")
            .map(|v| builder.parent_resource_id(v));

        builder
    }

    fn build(&mut self) -> Result<Self::Accessor> {
        let root = normalize_root(
            self.root
                .clone()
                .unwrap_or_else(|| "/".to_string())
                .as_str(),
        );

        let endpoint = self.endpoint.clone().unwrap();
        let parent_resource_id = self.parent_resource_id.clone().unwrap();

        let agent = Agent {
            private_key: self.private_key.clone(),
            public_key: self.public_key.clone().unwrap(),
            subject: format!("{}/agents/{}", endpoint, self.public_key.clone().unwrap()),
            created_at: 1,
            name: Some("agent".to_string()),
        };

        Ok(AtomicserverBackend::new(Adapter {
            parent_resource_id,
            endpoint,
            agent,
            client: HttpClient::new().map_err(|err| {
                err.with_operation("Builder::build")
                    .with_context("service", Scheme::Atomicserver)
            })?,
        })
        .with_root(&root))
    }
}

/// Backend for Atomicserver services.
pub type AtomicserverBackend = kv::Backend<Adapter>;

const FILENAME_PROPERTY: &str = "https://atomicdata.dev/properties/filename";

#[derive(Debug, Serialize)]
struct CommitStruct {
    #[serde(rename = "https://atomicdata.dev/properties/createdAt")]
    created_at: i64,
    #[serde(rename = "https://atomicdata.dev/properties/destroy")]
    destroy: bool,
    #[serde(rename = "https://atomicdata.dev/properties/isA")]
    is_a: Vec<String>,
    #[serde(rename = "https://atomicdata.dev/properties/signer")]
    signer: String,
    #[serde(rename = "https://atomicdata.dev/properties/subject")]
    subject: String,
}

#[derive(Debug, Serialize)]
struct CommitStructSigned {
    #[serde(rename = "https://atomicdata.dev/properties/createdAt")]
    created_at: i64,
    #[serde(rename = "https://atomicdata.dev/properties/destroy")]
    destroy: bool,
    #[serde(rename = "https://atomicdata.dev/properties/isA")]
    is_a: Vec<String>,
    #[serde(rename = "https://atomicdata.dev/properties/signature")]
    signature: String,
    #[serde(rename = "https://atomicdata.dev/properties/signer")]
    signer: String,
    #[serde(rename = "https://atomicdata.dev/properties/subject")]
    subject: String,
}

#[derive(Debug, Deserialize)]
struct FileStruct {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "https://atomicdata.dev/properties/downloadURL")]
    download_url: String,
}

#[derive(Debug, Deserialize)]
struct QueryResultStruct {
    #[serde(
        rename = "https://atomicdata.dev/properties/endpoint/results",
        default = "empty_vec"
    )]
    results: Vec<FileStruct>,
}

fn empty_vec() -> Vec<FileStruct> {
    Vec::new()
}

#[derive(Clone)]
pub struct Adapter {
    parent_resource_id: String,
    endpoint: String,
    agent: Agent,
    client: HttpClient,
}

impl Debug for Adapter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut ds = f.debug_struct("Adapter");
        ds.finish()
    }
}

impl Adapter {
    fn sign(&self, url: &str, mut req: http::request::Builder) -> http::request::Builder {
        let auth_headers = get_authentication_headers(url, &self.agent)
            .map_err(|err| {
                Error::new(
                    ErrorKind::Unexpected,
                    "Failed to get authentication headers",
                )
                .with_context("service", Scheme::Atomicserver)
                .set_source(err)
            })
            .unwrap();

        for (k, v) in &auth_headers {
            req = req.header(k, v);
        }

        req
    }
}

impl Adapter {
    pub fn atomic_get_object_request(&self, path: &str) -> Result<Request<AsyncBody>> {
        let path = normalize_path(path);
        let path = path.as_str();

        let filename_property_escaped = FILENAME_PROPERTY.replace(':', "\\:").replace('.', "\\.");
        let url = format!(
            "{}/search?filters={}:%22{}%22",
            self.endpoint,
            percent_encode_path(&filename_property_escaped),
            percent_encode_path(path)
        );

        let mut req = Request::get(&url);
        req = self.sign(&url, req);
        req = req.header(http::header::ACCEPT, "application/ad+json");

        let req = req
            .body(AsyncBody::Empty)
            .map_err(new_request_build_error)?;

        Ok(req)
    }

    async fn atomic_post_object_request(
        &self,
        path: &str,
        value: &[u8],
    ) -> Result<Request<AsyncBody>> {
        let path = normalize_path(path);
        let path = path.as_str();

        let url = format!(
            "{}/upload?parent={}",
            self.endpoint,
            percent_encode_path(&self.parent_resource_id)
        );

        let mut req = Request::post(&url);
        req = self.sign(&url, req);

        let datapart = FormDataPart::new("assets")
            .header(
                CONTENT_DISPOSITION,
                format!("form-data; name=\"assets\"; filename=\"{}\"", path)
                    .parse()
                    .unwrap(),
            )
            .header(CONTENT_TYPE, "text/plain".parse().unwrap())
            .content(value.to_vec());

        let multipart = Multipart::new().part(datapart);
        let req = multipart.apply(req)?;

        Ok(req)
    }

    pub fn atomic_delete_object_request(&self, subject: &str) -> Result<Request<AsyncBody>> {
        let url = format!("{}/commit", self.endpoint);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("You're a time traveler")
            .as_millis() as i64;

        let commit_to_sign = CommitStruct {
            created_at: timestamp,
            destroy: true,
            is_a: ["https://atomicdata.dev/classes/Commit".to_string()].to_vec(),
            signer: self.agent.subject.to_string(),
            subject: subject.to_string().clone(),
        };
        let commit_sign_string =
            serde_json::to_string(&commit_to_sign).map_err(new_json_serialize_error)?;

        let signature = sign_message(
            &commit_sign_string,
            self.agent.private_key.as_ref().unwrap(),
            &self.agent.public_key,
        )
        .unwrap();

        let commit = CommitStructSigned {
            created_at: timestamp,
            destroy: true,
            is_a: ["https://atomicdata.dev/classes/Commit".to_string()].to_vec(),
            signature,
            signer: self.agent.subject.to_string(),
            subject: subject.to_string().clone(),
        };

        let req = Request::post(&url);
        let body_string = serde_json::to_string(&commit).map_err(new_json_serialize_error)?;

        let body_bytes = body_string.as_bytes().to_owned();
        let req = req
            .body(AsyncBody::Bytes(body_bytes.into()))
            .map_err(new_request_build_error)?;

        Ok(req)
    }

    pub async fn download_from_url(&self, download_url: &String) -> Result<Bytes> {
        let req = Request::get(download_url);
        let req = req
            .body(AsyncBody::Empty)
            .map_err(new_request_build_error)?;
        let resp = self.client.send(req).await?;
        let bytes_file = resp.into_body().bytes().await?;

        Ok(bytes_file)
    }
}

impl Adapter {
    async fn wait_for_resource(&self, path: &str, expect_exist: bool) -> Result<()> {
        // This is used to wait until insert/delete is actually effective
        // This wait function is needed because atomicserver commits are not processed in real-time
        // See https://docs.atomicdata.dev/commits/intro.html#motivation
        for _i in 0..1000 {
            let req = self.atomic_get_object_request(path)?;
            let resp = self.client.send(req).await?;
            let bytes = resp.into_body().bytes().await?;
            let query_result: QueryResultStruct =
                serde_json::from_str(std::str::from_utf8(&bytes).unwrap())
                    .map_err(new_json_deserialize_error)?;
            if !expect_exist && query_result.results.is_empty() {
                break;
            }
            if expect_exist && !query_result.results.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        }

        Ok(())
    }
}

#[async_trait]
impl kv::Adapter for Adapter {
    fn metadata(&self) -> kv::Metadata {
        kv::Metadata::new(
            Scheme::Atomicserver,
            "atomicserver",
            Capability {
                read: true,
                write: true,
                delete: true,
                create_dir: true,
                ..Default::default()
            },
        )
    }

    async fn get(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let req = self.atomic_get_object_request(path)?;
        let resp = self.client.send(req).await?;
        let bytes = resp.into_body().bytes().await?;

        let query_result: QueryResultStruct =
            serde_json::from_str(std::str::from_utf8(&bytes).unwrap())
                .map_err(new_json_deserialize_error)?;

        if query_result.results.is_empty() {
            return Err(Error::new(
                ErrorKind::NotFound,
                "atomicserver: key not found",
            ));
        }

        let bytes_file = self
            .download_from_url(&query_result.results[0].download_url)
            .await?;

        Ok(Some(bytes_file.to_vec()))
    }

    async fn set(&self, path: &str, value: &[u8]) -> Result<()> {
        let req = self.atomic_get_object_request(path)?;
        let res = self.client.send(req).await?;
        let bytes = res.into_body().bytes().await?;

        let query_result: QueryResultStruct =
            serde_json::from_str(std::str::from_utf8(&bytes).unwrap())
                .map_err(new_json_deserialize_error)?;

        for result in query_result.results {
            let req = self.atomic_delete_object_request(&result.id)?;
            let _res = self.client.send(req).await?;
        }

        let _ = self.wait_for_resource(path, false).await;

        let req = self.atomic_post_object_request(path, value).await?;
        let _res = self.client.send(req).await?;
        let _ = self.wait_for_resource(path, true).await;

        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let req = self.atomic_get_object_request(path)?;
        let res = self.client.send(req).await?;
        let bytes = res.into_body().bytes().await?;

        let query_result: QueryResultStruct =
            serde_json::from_str(std::str::from_utf8(&bytes).unwrap())
                .map_err(new_json_deserialize_error)?;

        for result in query_result.results {
            let req = self.atomic_delete_object_request(&result.id)?;
            let _res = self.client.send(req).await?;
        }

        let _ = self.wait_for_resource(path, false).await;

        Ok(())
    }
}
