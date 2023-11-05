use poem::{listener::TcpListener, web::Data, EndpointExt, Result, Route, Server};
use poem_openapi::{param::Query, payload::PlainText, OpenApi, OpenApiService};
use poem_openapi::{payload::Json, ApiResponse, Object, Tags};
mod settings;
use settings::Settings;

use terraphim_config::Config;

#[derive(Tags)]
enum ApiTags {
    /// Config operations
    Config,
}


#[derive(Default)]
struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {name}!")),
            None => PlainText("hello!".to_string()),
        }
    }
    #[oai(path = "/config", method = "get")]
    async fn config(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {name}!")),
            None => PlainText("hello!".to_string()),
        }
    }

}#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let settings = Settings::new().unwrap();
    println!("{:?}", settings);
    let bind_addr = settings.server_url.clone();
    let api_endpoint = settings.api_endpoint.clone();
    let api_service = OpenApiService::new(Api, "Hello World", "1.0").server(api_endpoint);
    let ui = api_service.swagger_ui();
    let spec = api_service.spec();
    let route = Route::new()
        .nest("/api", api_service)
        .nest("/doc", ui)
        .at("/spec", poem::endpoint::make_sync(move |_| spec.clone()))
        // .with(Cors::new())
        .data(settings);

    Server::new(TcpListener::bind(bind_addr)).run(route).await?;

    Ok(())
}
