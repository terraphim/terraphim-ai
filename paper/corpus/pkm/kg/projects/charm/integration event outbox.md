# Integration Event Outbox

Transactional outbox pattern for publishing canonical domain events (loan_activated, activation_blocked, etc.) to downstream consumers. Tracks per-consumer publish attempts with idempotency keys.

synonyms:: event outbox, outbox
