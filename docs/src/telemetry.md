---
title: Telemetry
description: "Client telemetry in this fork."
---

# Telemetry in this fork

This fork does not create, queue, save, or upload client usage events or crash telemetry. The `telemetry::event!` macro does not evaluate event names or properties. No telemetry log viewer is available.

The existing `telemetry.metrics` and `telemetry.diagnostics` settings do not enable collection or uploads. You do not need to change these settings to stop client telemetry.

Local logs and diagnostic tools remain available for your own use. They are not client telemetry uploads. Network requests to services you choose to use, such as AI providers, are separate from client telemetry.
