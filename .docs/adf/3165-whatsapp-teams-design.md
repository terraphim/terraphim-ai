# Issue #3165 Corrective Design: WhatsApp + Microsoft Teams Channels

## Scope

Implement first-class TinyClaw channel adapters for the two explicit Wave 4 first candidates: WhatsApp Cloud API and Microsoft Teams Bot Framework. Existing Wave 4 stubs remain unchanged except for shared channel registration.

## WhatsApp Cloud API

### Outbound Schema

Endpoint: `POST {graph_base_url}/{api_version}/{phone_number_id}/messages`

Headers:
- `Authorization: Bearer {access_token}`
- `Content-Type: application/json`

Body per text chunk:

```json
{
  "messaging_product": "whatsapp",
  "recipient_type": "individual",
  "to": "15551234567",
  "type": "text",
  "text": {
    "preview_url": false,
    "body": "message text"
  }
}
```

### Inbound Schema

Accept Meta webhook JSON containing `entry[].changes[].value.messages[]`. Text messages produce `InboundMessage { channel: "whatsapp", sender_id: from, chat_id: from, content: text.body }`. Message IDs and contact profile names are copied into metadata. Unsupported message types are ignored rather than coerced into fake text.

### Security

Webhook verification has two paths:
- GET subscription challenge checks `hub.mode=subscribe` and exact `hub.verify_token`.
- POST body validation checks `X-Hub-Signature-256: sha256=<hex>` with HMAC-SHA256 over the raw body using `app_secret`. Comparison uses `hmac::Mac::verify_slice`.

Secrets are redacted in `Debug`.

### Chunking, Retries, Idempotency

WhatsApp Cloud API allows long text messages, but TinyClaw caps chunks at 4,096 bytes for deterministic cross-channel behavior and to avoid provider-side rejection. Sends are retried with exponential backoff for transient HTTP status codes 408, 409, 425, 429, and 5xx. Inbound idempotency uses the provider message ID; duplicate IDs can be dropped by the HTTP webhook layer before forwarding to the bus.

## Microsoft Teams

### Outbound Schema

Token endpoint: configurable, defaulting to the Bot Framework OAuth client credentials endpoint.

Endpoint: `POST {service_url}/v3/conversations/{conversation_id}/activities`

Headers:
- `Authorization: Bearer {bot_framework_access_token}`
- `Content-Type: application/json`

Body per text chunk:

```json
{
  "type": "message",
  "text": "message text"
}
```

The adapter mints a Bot Framework access token with `client_id`, `client_secret`, `scope=https://api.botframework.com/.default`, and `grant_type=client_credentials`. Tests use a hermetic token endpoint fixture; production does not fake sends.

### Inbound Schema

Accept Bot Framework Activity JSON with `type=message`. It maps `from.id` to `sender_id`, `conversation.id` to `chat_id`, `text` to `content`, and copies activity ID, service URL, channel ID, and tenant ID into metadata.

### Security

Production webhook handlers must validate the inbound `Authorization: Bearer <JWT>` token against Microsoft Bot Framework OpenID metadata before trusting an activity. The channel module exposes parsing and outbound contracts; the deployable HTTP route must call the validation hook before publishing to the bus.

### Chunking, Retries, Idempotency

Teams text is capped at 28,000 bytes per activity. Sends are retried with exponential backoff for transient HTTP status codes 408, 409, 425, 429, and 5xx. Inbound idempotency uses `activity.id`; duplicate activity IDs can be dropped by the webhook layer before forwarding to the bus.

## External Prerequisites

WhatsApp live test:
- `TERRAPHIM_TEST_LIVE=1`
- `WHATSAPP_ACCESS_TOKEN`
- `WHATSAPP_PHONE_NUMBER_ID`
- `WHATSAPP_TEST_RECIPIENT`

Teams live test:
- `TERRAPHIM_TEST_LIVE=1`
- `TEAMS_APP_ID`
- `TEAMS_APP_PASSWORD`
- `TEAMS_TEST_SERVICE_URL`
- `TEAMS_TEST_CONVERSATION_ID`

Live tests are `#[ignore]` and never run in hermetic CI by default.

## Recommended Child Issue Split

- WhatsApp Cloud API channel hardening: webhook HTTP route integration, duplicate cache, media message ingestion.
- Microsoft Teams Bot Framework channel hardening: OpenID/JWT validation route integration, proactive conversation references, tenant allowlist controls.
