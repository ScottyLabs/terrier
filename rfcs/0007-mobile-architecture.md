# RFC 0007: Mobile Architecture

- **Status:** Draft
- **Author(s):** @kritdass
- **Created:** 2026-03-05
- **Updated:** 2026-03-05

## Overview

This RFC establishes the core framework used for the Terrier mobile application (iOS and Android): Tauri 2.0.

## Motivation

Providing Terrier as a native mobile application makes it easier for attendees, organizers, judges, and sponsors to interact with the platform during a hackathon. Many key hackathon interactions require access to native device APIs, such as scanning QR codes, receiving real-time event announcements, and using location-based-features. We also want to avoid duplicating our effort. Maintaining a separate codebase exclusively for a Terrier mobile app would be tedious and problematic.

## Goals

- Establish mobile architecture framework
- Minimize extra effort to maintain a mobile app
- Provide support for essential native features: deep links, QR codes, push notifications, and geolocation
- Ensure compatibility with our existing infrastructure

## Non-Goals

- Replacing the framework of the Terrier web app
- Shipping Terrier as a desktop application
- Distribute Terrier solely as a PWA

## Detailed Design

Terrier Mobile will exist as a component within the monorepo, sharing the same stack as the web application to ensure UI parity. The mobile client wraps the web layer using Tauri 2.0, targeting both iOS and Android. While the majority of the logic will remain in the Svelte frontend or Rust backend, the architecture allows for direct access to native APIs via Switft and Kotlin through Tauri's plugin system when such low-level access (beyond existing plugins) is required.

Tauri has the advantages of providing security, resource efficiency, and flexible architecture. Moreover, the following Tauri plugins allow support for essential native features: [`barcode-scanner`](https://tauri.app/plugin/barcode-scanner/), [`deep-link`](https://tauri.app/plugin/deep-linking/), [`notification`](https://tauri.app/plugin/notification/), and [`geolocation`](https://tauri.app/plugin/geolocation/).

### Nix Build

Like `terrierDocs` and `terrierApp`, we use the same `bun2nix` to package the Terrier mobile app as a Nix derivation.

## Alternatives Considered

- **Capacitor**: A very strong contender that would also allow us to reuse code from our web app. Rejected because Tauri provides better security, higher performance, and has stable Rust bindings.
- **React Native**: The most popular framework for cross-platform mobile apps. Rejected because it would force us to rewrite our web app in React.
- **Flutter**: Rejected for similar reasons as React Native.
- **Separate Native Applications (Swift/Kotlin)**: Would provide high performance and direct access to mobile APIs, but we would have to maintain two more codebases. Tauri allows us to call native APIs using Swift/Kotlin anyways.

## Open Questions

- Should we provide a generic Terrier app or should individual hackathons use our platform to create and distribute their own apps for each hackathon?
