# RFC 0006: Observability

- **Status:** Draft
- **Author(s):** @kritdass, @sreeram
- **Created:** 2026-02-21
- **Updated:** 2026-02-21

## Overview

We will implement observability features to enhance the monitoring and debugging capabilities of Terrier. We will use Restate for logging, Prometheus for metrics, and OpenTelemetry with Jaeger for tracing

## Motivation

Terrier is a distributed system that requires monitoring and debugging capabilities to ensure its reliability. By implementing observability features, we can gain insights into the system's behavior and identify issues early.

## Goals

- Integrate Restate for logging
- Use Prometheus for metrics
- Use OpenTelemetry with Jaeger for tracing

## Non-Goals

- User telemetry
- Performance optimization

## Detailed Design

The meat of the RFC. Explain the design in enough detail that:

- Its interaction with other features is clear
- It's reasonably clear how to implement
- Corner cases are discussed

Include code examples, diagrams, or data models where helpful.

## Alternatives Considered

- Temporal
- Rust SDK is prerelease, not suitable for production use
- Datadog
- Not free for production use
- Honeycomb
- Not free for production use

## Open Questions

- How to integrate observability features with other parts of the Terrier system?
- How to configure and manage observability components?

## Implementation Phases

1. Integrate Restate for logging
1. Use Prometheus for metrics
1. Use OpenTelemetry with Jaeger for tracing
