use std::path::Path;
use terraphim_types::{Document, Index};
extern crate google_gmail1 as gmail1;
use super::{hash_as_string, IndexMiddleware};
use crate::{Error, Result};
use gmail1::api::Message;
use gmail1::{hyper, hyper_rustls, oauth2, Gmail};
use std::default::Default;

/// Middleware that uses Gmail API to index Email haystack.
#[derive(Default)]
pub struct GmailIndexer {
    // // E-Mail address of the user
    // user_id: String,
}

impl IndexMiddleware for GmailIndexer {
    /// Index the email haystack using Gmail API and return an index of documents
    ///
    /// # Errors
    ///
    /// Returns an error if the middleware fails to index the haystack
    async fn index(&self, needle: &str, haystack: &Path) -> Result<Index> {
        log::debug!("Indexing email haystack using Gmail API");
        // Get an ApplicationSecret instance by some means. It contains the `client_id` and
        // `client_secret`, among other things.
        let secret: oauth2::ApplicationSecret = Default::default();
        // Instantiate the authenticator. It will choose a suitable authentication flow for you,
        // unless you replace `None` with the desired Flow.
        // Provide your own `AuthenticatorDelegate` to adjust the way it operates and get feedback about
        // what's going on. You probably want to bring in your own `TokenStorage` to persist tokens and
        // retrieve them from storage.
        let auth = oauth2::InstalledFlowAuthenticator::builder(
            secret,
            oauth2::InstalledFlowReturnMethod::HTTPRedirect,
        )
        .build()
        .await?;

        log::debug!("Authenticating with Gmail API");

        let hub = Gmail::new(
            hyper::Client::builder().build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .https_or_http()
                    .enable_http1()
                    .build(),
            ),
            auth,
        );

        log::debug!("Fetching messages from Gmail API");

        let result = hub
            .users()
            // .messages_list(user_id)
            .messages_list("me")
            .q(needle)
            .max_results(55)
            .include_spam_trash(false)
            .doit()
            .await
            .map_err(|e| {
                Error::Indexation(format!("Failed to index the email haystack: {:?}", e))
            })?;

        log::debug!("Retrieved the following messages: {:?}", result.1);

        let mut index: Index = Index::default();

        let Some(messages) = result.1.messages else {
            return Ok(index);
        };

        for message in messages {
            let mut document: Document = Document::default();
            document.id = hash_as_string(&message.id);
            document.title = message.snippet.clone().unwrap_or_default();
            document.url = message.id.clone().unwrap_or_default();
            index.insert(document.id.to_string(), document.clone());
        }

        log::debug!("Indexed the following documents: {:?}", index);

        Ok(index)
    }
}
