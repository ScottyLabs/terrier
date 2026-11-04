# RFC 0007: Mobile Architecture

- **Status:** Accepted
- **Author(s):** @kritdass
- **Created:** 2026-03-05
- **Updated:** 2026-11-04

## Overview

This RFC establishes the core framework used for the Terrier mobile application (iOS and Android): [Capacitor](https://capacitorjs.com/docs).

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

Terrier Mobile will exist as a component within the monorepo, sharing the Svelte frontend with the web application. Capacitor wraps that web layer for iOS and Android, while the Rust backend remains a separate server accessed through the same HTTP API as the web client. Using Capacitor aligns Terrier with other ScottyLabs mobile applications, including o-quest and the planned maps app.

Native capabilities use [Capacitor plugins](https://capacitorjs.com/docs/plugins), with custom Swift and Java/Kotlin integration where needed. Deep links use the [App plugin](https://capacitorjs.com/docs/apis/app); push notifications and geolocation use the corresponding [Push Notifications](https://capacitorjs.com/docs/apis/push-notifications) and [Geolocation](https://capacitorjs.com/docs/apis/geolocation) plugins. QR scanning also requires a Capacitor-compatible integration; select and validate it during mobile implementation.

### App Distribution and Updates

- **Single Official App**: Publish one official Terrier app for iOS and Android. Users connect to their hackathon's instance by entering an instance URL, and the app fetches its branding and data.
- **Updating Mechanism**: Distribute native wrapper and plugin changes through the Apple App Store and Google Play Store. The web-layer update mechanism and client/server compatibility checks remain open. A backend deployment alone does not establish that an installed mobile client has received updated web assets or native plugins.

### Build and Packaging

Build the shared frontend using the repository's Deno workflow and synchronize its output into the Capacitor projects using the [Capacitor development workflow](https://capacitorjs.com/docs/basics/workflow). Declare shared tooling in `devenv.nix` and use Kennel where its build helpers support the artifact. Native builds require platform tooling, including Xcode for iOS and the Android SDK for Android, as described in [Capacitor environment setup](https://capacitorjs.com/docs/getting-started/environment-setup). Signing and store distribution need platform-specific release jobs.

## Alternatives Considered

- **Tauri**: Supports sharing the web frontend and native integrations. Capacitor is selected to align mobile tooling and integration patterns across ScottyLabs applications.
- **React Native**: Would require replacing the shared Svelte UI with a React-based mobile UI.
- **Flutter**: Would require a separate UI implementation rather than reusing the Svelte frontend.
- **Separate Native Applications (Swift/Kotlin)**: Would require maintaining two native UI codebases. Capacitor plugins provide native integration while preserving the shared frontend.

## Open Questions

- How should the mobile web layer be delivered and updated: bundled releases, downloaded updates, or an instance-hosted web layer?
- How should installed clients detect incompatibility with an instance's unversioned API and obtain an update? Coordinate this with RFC 0011.
- Which QR scanning integration should be used, and what native build, signing, and store-release automation is required?
