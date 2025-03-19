use serde::Serialize;

/// OpenAPI documentation for the MCP server
#[derive(Serialize)]
pub struct ApiDoc {
    openapi: String,
    info: ApiInfo,
    paths: ApiPaths,
    components: ApiComponents,
}

#[derive(Serialize)]
struct ApiInfo {
    title: String,
    description: String,
    version: String,
}

#[derive(Serialize)]
struct ApiPaths {
    #[serde(rename = "/v1/resources")]
    resources: ResourcePaths,
    #[serde(rename = "/v1/resources/subscribe")]
    subscribe: SubscribePaths,
    #[serde(rename = "/v1/resources/templates")]
    templates: TemplatePaths,
    #[serde(rename = "/v1/capabilities")]
    capabilities: CapabilitiesPaths,
}

#[derive(Serialize)]
struct ResourcePaths {
    get: Operation,
    post: Operation,
}

#[derive(Serialize)]
struct SubscribePaths {
    post: Operation,
    delete: Operation,
}

#[derive(Serialize)]
struct TemplatePaths {
    get: Operation,
}

#[derive(Serialize)]
struct CapabilitiesPaths {
    get: Operation,
}

#[derive(Serialize)]
struct Operation {
    summary: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<Vec<Parameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_body: Option<RequestBody>,
    responses: Responses,
    tags: Vec<String>,
}

#[derive(Serialize)]
struct Parameter {
    name: String,
    #[serde(rename = "in")]
    location: String,
    description: String,
    required: bool,
    schema: Schema,
}

#[derive(Serialize)]
struct RequestBody {
    description: String,
    required: bool,
    content: Content,
}

#[derive(Serialize)]
struct Content {
    #[serde(rename = "application/json")]
    json: MediaType,
}

#[derive(Serialize)]
struct MediaType {
    schema: Schema,
}

#[derive(Serialize)]
struct Schema {
    #[serde(rename = "type")]
    schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
}

#[derive(Serialize)]
struct Responses {
    #[serde(rename = "200")]
    success: Response,
    #[serde(rename = "400")]
    bad_request: Response,
    #[serde(rename = "404")]
    not_found: Response,
    #[serde(rename = "500")]
    internal_error: Response,
}

#[derive(Serialize)]
struct Response {
    description: String,
    content: Option<Content>,
}

#[derive(Serialize)]
struct ApiComponents {
    schemas: Schemas,
}

#[derive(Serialize)]
struct Schemas {
    Resource: ComponentSchema,
    ResourceContent: ComponentSchema,
    ResourceList: ComponentSchema,
    ResourceTemplate: ComponentSchema,
    ResourceCapabilities: ComponentSchema,
    Error: ComponentSchema,
}

#[derive(Serialize)]
struct ComponentSchema {
    #[serde(rename = "type")]
    schema_type: String,
    properties: serde_json::Value,
    required: Vec<String>,
}

impl Default for ApiDoc {
    fn default() -> Self {
        Self {
            openapi: "3.0.3".to_string(),
            info: ApiInfo {
                title: "Model Context Protocol API".to_string(),
                description: "API for the Model Context Protocol (MCP) server".to_string(),
                version: "1.0.0".to_string(),
            },
            paths: ApiPaths {
                resources: ResourcePaths {
                    get: Operation {
                        summary: "List available resources".to_string(),
                        description: "Returns a paginated list of available resources".to_string(),
                        parameters: Some(vec![
                            Parameter {
                                name: "cursor".to_string(),
                                location: "query".to_string(),
                                description: "Pagination cursor".to_string(),
                                required: false,
                                schema: Schema {
                                    schema_type: "string".to_string(),
                                    format: None,
                                    reference: None,
                                },
                            },
                            Parameter {
                                name: "limit".to_string(),
                                location: "query".to_string(),
                                description: "Maximum number of items to return".to_string(),
                                required: false,
                                schema: Schema {
                                    schema_type: "integer".to_string(),
                                    format: Some("int32".to_string()),
                                    reference: None,
                                },
                            },
                        ]),
                        request_body: None,
                        responses: Responses {
                            success: Response {
                                description: "List of resources".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/ResourceList".to_string()),
                                        },
                                    },
                                }),
                            },
                            bad_request: Response {
                                description: "Invalid request parameters".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                            not_found: Response {
                                description: "Resource not found".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                            internal_error: Response {
                                description: "Internal server error".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                        },
                        tags: vec!["Resources".to_string()],
                    },
                    post: Operation {
                        summary: "Read resource contents".to_string(),
                        description: "Returns the contents of a specific resource".to_string(),
                        parameters: None,
                        request_body: Some(RequestBody {
                            description: "Resource read request".to_string(),
                            required: true,
                            content: Content {
                                json: MediaType {
                                    schema: Schema {
                                        schema_type: "object".to_string(),
                                        format: None,
                                        reference: None,
                                    },
                                },
                            },
                        }),
                        responses: Responses {
                            success: Response {
                                description: "Resource contents".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/ResourceContent".to_string()),
                                        },
                                    },
                                }),
                            },
                            bad_request: Response {
                                description: "Invalid request parameters".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                            not_found: Response {
                                description: "Resource not found".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                            internal_error: Response {
                                description: "Internal server error".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                        },
                        tags: vec!["Resources".to_string()],
                    },
                },
                subscribe: SubscribePaths {
                    post: Operation {
                        summary: "Subscribe to resource changes".to_string(),
                        description: "Subscribe to notifications for changes to a specific resource".to_string(),
                        parameters: None,
                        request_body: Some(RequestBody {
                            description: "Subscription request".to_string(),
                            required: true,
                            content: Content {
                                json: MediaType {
                                    schema: Schema {
                                        schema_type: "object".to_string(),
                                        format: None,
                                        reference: None,
                                    },
                                },
                            },
                        }),
                        responses: Responses {
                            success: Response {
                                description: "Successfully subscribed".to_string(),
                                content: None,
                            },
                            bad_request: Response {
                                description: "Invalid request parameters".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                            not_found: Response {
                                description: "Resource not found".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                            internal_error: Response {
                                description: "Internal server error".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                        },
                        tags: vec!["Subscriptions".to_string()],
                    },
                    delete: Operation {
                        summary: "Unsubscribe from resource changes".to_string(),
                        description: "Unsubscribe from notifications for a specific resource".to_string(),
                        parameters: None,
                        request_body: Some(RequestBody {
                            description: "Unsubscribe request".to_string(),
                            required: true,
                            content: Content {
                                json: MediaType {
                                    schema: Schema {
                                        schema_type: "object".to_string(),
                                        format: None,
                                        reference: None,
                                    },
                                },
                            },
                        }),
                        responses: Responses {
                            success: Response {
                                description: "Successfully unsubscribed".to_string(),
                                content: None,
                            },
                            bad_request: Response {
                                description: "Invalid request parameters".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                            not_found: Response {
                                description: "Resource not found".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                            internal_error: Response {
                                description: "Internal server error".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                        },
                        tags: vec!["Subscriptions".to_string()],
                    },
                },
                templates: TemplatePaths {
                    get: Operation {
                        summary: "List available resource templates".to_string(),
                        description: "Returns a list of available resource templates".to_string(),
                        parameters: None,
                        request_body: None,
                        responses: Responses {
                            success: Response {
                                description: "List of templates".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "array".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/ResourceTemplate".to_string()),
                                        },
                                    },
                                }),
                            },
                            bad_request: Response {
                                description: "Invalid request parameters".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                            not_found: Response {
                                description: "Resource not found".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                            internal_error: Response {
                                description: "Internal server error".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                        },
                        tags: vec!["Templates".to_string()],
                    },
                },
                capabilities: CapabilitiesPaths {
                    get: Operation {
                        summary: "Get server capabilities".to_string(),
                        description: "Returns the server's capabilities".to_string(),
                        parameters: None,
                        request_body: None,
                        responses: Responses {
                            success: Response {
                                description: "Server capabilities".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/ResourceCapabilities".to_string()),
                                        },
                                    },
                                }),
                            },
                            bad_request: Response {
                                description: "Invalid request parameters".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                            not_found: Response {
                                description: "Resource not found".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                            internal_error: Response {
                                description: "Internal server error".to_string(),
                                content: Some(Content {
                                    json: MediaType {
                                        schema: Schema {
                                            schema_type: "object".to_string(),
                                            format: None,
                                            reference: Some("#/components/schemas/Error".to_string()),
                                        },
                                    },
                                }),
                            },
                        },
                        tags: vec!["Server".to_string()],
                    },
                },
            },
            components: ApiComponents {
                schemas: Schemas {
                    Resource: ComponentSchema {
                        schema_type: "object".to_string(),
                        properties: serde_json::json!({
                            "uri": {
                                "type": "string",
                                "format": "uri"
                            },
                            "name": {
                                "type": "string"
                            },
                            "description": {
                                "type": "string"
                            },
                            "mimeType": {
                                "type": "string"
                            }
                        }),
                        required: vec!["uri".to_string(), "name".to_string()],
                    },
                    ResourceContent: ComponentSchema {
                        schema_type: "object".to_string(),
                        properties: serde_json::json!({
                            "uri": {
                                "type": "string",
                                "format": "uri"
                            },
                            "mimeType": {
                                "type": "string"
                            },
                            "content": {
                                "oneOf": [
                                    {
                                        "type": "string"
                                    },
                                    {
                                        "type": "string",
                                        "format": "byte"
                                    }
                                ]
                            }
                        }),
                        required: vec!["uri".to_string(), "mimeType".to_string(), "content".to_string()],
                    },
                    ResourceList: ComponentSchema {
                        schema_type: "object".to_string(),
                        properties: serde_json::json!({
                            "resources": {
                                "type": "array",
                                "items": {
                                    "$ref": "#/components/schemas/Resource"
                                }
                            },
                            "nextCursor": {
                                "type": "string"
                            }
                        }),
                        required: vec!["resources".to_string()],
                    },
                    ResourceTemplate: ComponentSchema {
                        schema_type: "object".to_string(),
                        properties: serde_json::json!({
                            "uriTemplate": {
                                "type": "string"
                            },
                            "name": {
                                "type": "string"
                            },
                            "description": {
                                "type": "string"
                            },
                            "mimeType": {
                                "type": "string"
                            }
                        }),
                        required: vec!["uriTemplate".to_string(), "name".to_string()],
                    },
                    ResourceCapabilities: ComponentSchema {
                        schema_type: "object".to_string(),
                        properties: serde_json::json!({
                            "subscribe": {
                                "type": "boolean"
                            },
                            "listChanged": {
                                "type": "boolean"
                            }
                        }),
                        required: vec!["subscribe".to_string(), "listChanged".to_string()],
                    },
                    Error: ComponentSchema {
                        schema_type: "object".to_string(),
                        properties: serde_json::json!({
                            "error": {
                                "type": "string"
                            },
                            "details": {
                                "type": "string"
                            }
                        }),
                        required: vec!["error".to_string()],
                    },
                },
            },
        }
    }
}

/// Returns the OpenAPI documentation as JSON
pub fn get_api_docs() -> String {
    serde_json::to_string_pretty(&ApiDoc::default()).unwrap()
}

pub fn get_swagger_ui() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="description" content="Terraphim MCP Server API Documentation" />
    <title>Terraphim MCP Server API Documentation</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.11.0/swagger-ui.css" />
    <style>
        body {
            margin: 0;
            padding: 0;
        }
        .swagger-ui .topbar {
            background-color: #1b1b1b;
            padding: 10px 0;
        }
        .swagger-ui .info {
            margin: 20px 0;
        }
        .swagger-ui .info .title {
            color: #3b4151;
        }
        .swagger-ui .scheme-container {
            box-shadow: none;
            border-bottom: 1px solid #eee;
        }
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5.11.0/swagger-ui-bundle.js" crossorigin></script>
    <script src="https://unpkg.com/swagger-ui-dist@5.11.0/swagger-ui-standalone-preset.js" crossorigin></script>
    <script>
        window.onload = () => {
            window.ui = SwaggerUIBundle({
                url: '/openapi.json',
                dom_id: '#swagger-ui',
                deepLinking: true,
                displayOperationId: true,
                displayRequestDuration: true,
                defaultModelsExpandDepth: 3,
                defaultModelExpandDepth: 3,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout",
                syntaxHighlight: {
                    activated: true,
                    theme: "monokai"
                }
            });
        };
    </script>
</body>
</html>"#.to_string()
} 