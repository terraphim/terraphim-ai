---
id: push-notification-architecture
title: Push Notification Architecture
---

# Push Notification Architecture

A pattern for delivering real-time notifications to mobile devices via platform-specific services (Apple Push Notification Service / APNS, Google Cloud Messaging / GCM, Firebase Cloud Messaging / FCM).

Components:
- Device registration service (register, deregister, retrieve, update)
- Outbound management platform (message dispatch and templating)
- Customer device registry (linked to customer identity / SCV)
- Preference-based routing (customer opt-in per notification type)

Design principles:
- Device data owned internally, not by third-party platforms
- Decoupled from preference management for independent delivery
- Deep linking into app screens for actionable notifications
- Governance model for appropriate use and regulatory compliance

Related: preference centre, APNS, GCM, FCM, digital outbound management
