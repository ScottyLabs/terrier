# Terrier PWA iOS App

A native iOS wrapper for the [Terrier](https://terrier.scottylabs.org) Progressive Web App (PWA), providing a seamless native app experience while leveraging the PWA's web-based functionality.

## Features

- **Native PWA Wrapper**: Full WKWebView integration with PWA-specific enhancements
- **Service Worker Support**: App-bound domains enable service worker functionality for offline support
- **Pull-to-Refresh**: Native iOS pull-to-refresh gesture support
- **Native Dialogs**: JavaScript alerts, confirms, and prompts rendered as native iOS dialogs
- **Deep Linking**: Support for `terrier://` URL scheme and universal links
- **Native Sharing**: Bridge for sharing content using iOS share sheet
- **Haptic Feedback**: Native haptic feedback accessible from JavaScript
- **Offline Detection**: Network monitoring with offline status banner
- **Error Handling**: User-friendly error screens with retry functionality
- **Dark Mode**: Full dark mode support

## Requirements

- Xcode 15.0 or later
- iOS 15.0+ deployment target
- macOS Ventura or later (for development)
- Apple Developer account (for device deployment)

## Project Structure

```
pwa_ios/
├── TerrierPWA.xcodeproj/          # Xcode project file
│   ├── project.pbxproj
│   └── xcshareddata/
│       └── xcschemes/
│           └── TerrierPWA.xcscheme
├── TerrierPWA/
│   ├── TerrierPWAApp.swift        # App entry point
│   ├── ContentView.swift          # Main view with loading/error states
│   ├── PWAWebView.swift           # WKWebView wrapper with PWA support
│   ├── NetworkMonitor.swift       # Network connectivity monitoring
│   ├── Info.plist                 # App configuration
│   ├── TerrierPWA.entitlements    # App entitlements
│   └── Assets.xcassets/           # App icons and colors
│       ├── AppIcon.appiconset/
│       ├── AccentColor.colorset/
│       ├── LaunchScreenColor.colorset/
│       └── LaunchLogo.imageset/
└── README.md
```

## Getting Started

### 1. Open the Project

```bash
cd pwa_ios
open TerrierPWA.xcodeproj
```

### 2. Add App Icons

Before building, add your app icons to the asset catalog:

1. Open `Assets.xcassets` in Xcode
2. Select `AppIcon`
3. Add a 1024x1024 PNG image for the app icon
4. Optionally add dark mode and tinted variants

Required images for `AppIcon.appiconset`:
- `appicon-1024.png` (1024x1024) - Standard app icon
- `appicon-1024-dark.png` (1024x1024) - Dark mode variant (optional)
- `appicon-1024-tinted.png` (1024x1024) - Tinted variant (optional)

For the launch screen logo (`LaunchLogo.imageset`):
- `launch-logo.png` (1x scale)
- `launch-logo@2x.png` (2x scale)
- `launch-logo@3x.png` (3x scale)

### 3. Configure Signing

1. Select the `TerrierPWA` target in Xcode
2. Go to "Signing & Capabilities"
3. Select your Development Team
4. Xcode will automatically manage signing

### 4. Build and Run

- Select a simulator or connected device
- Press `Cmd + R` to build and run

## Configuration

### Changing the PWA URL

Edit [PWAWebView.swift](TerrierPWA/PWAWebView.swift#L8-L12):

```swift
struct PWAConfig {
    static let pwaURL = URL(string: "https://terrier.scottylabs.org")!
    static let allowedHosts = ["terrier.scottylabs.org", "scottylabs.org"]
    static let appName = "Terrier"
    static let backgroundColor = UIColor.systemBackground
}
```

### Adding More Allowed Domains

To allow navigation to additional domains without opening Safari, add them to `allowedHosts` in the `PWAConfig` struct.

### Customizing the Accent Color

Edit `Assets.xcassets/AccentColor.colorset/Contents.json` to change the app's accent color.

## JavaScript Bridge

The app exposes a JavaScript bridge for native functionality:

```javascript
// Check if running in native app
if (window.isNativeApp) {
    console.log('Running in Terrier iOS app');
}

// Share content using native share sheet
window.TerrierNative.share({
    text: 'Check out Terrier!',
    url: 'https://terrier.scottylabs.org'
});

// Trigger haptic feedback
window.TerrierNative.haptic('medium'); // light, medium, heavy, rigid, soft
```

## App Store Submission

### Before Submitting

1. **Update Version Numbers**: Edit `Info.plist` or project settings
   - `CFBundleShortVersionString` (e.g., "1.0.0")
   - `CFBundleVersion` (build number, e.g., "1")

2. **Add App Icons**: Ensure all icon variants are present

3. **Configure Privacy**: Update privacy usage descriptions in `Info.plist` if needed

4. **Set Development Team**: Configure proper signing with your distribution certificate

5. **Archive**: Product → Archive in Xcode

### Universal Links (Optional)

To enable universal links, add an `apple-app-site-association` file to your web server at `https://terrier.scottylabs.org/.well-known/apple-app-site-association`:

```json
{
    "applinks": {
        "apps": [],
        "details": [
            {
                "appID": "TEAM_ID.org.scottylabs.terrier",
                "paths": ["*"]
            }
        ]
    },
    "webcredentials": {
        "apps": ["TEAM_ID.org.scottylabs.terrier"]
    }
}
```

Replace `TEAM_ID` with your Apple Developer Team ID.

## Troubleshooting

### Service Workers Not Working

Ensure your domains are listed in `WKAppBoundDomains` in `Info.plist`:

```xml
<key>WKAppBoundDomains</key>
<array>
    <string>terrier.scottylabs.org</string>
    <string>scottylabs.org</string>
</array>
```

### Navigation to External Links

External links (not in `allowedHosts`) automatically open in Safari. To allow more domains, add them to `PWAConfig.allowedHosts`.

### Build Errors

1. Clean build folder: `Cmd + Shift + K`
2. Delete derived data: `rm -rf ~/Library/Developer/Xcode/DerivedData`
3. Restart Xcode

## License

This iOS wrapper is part of the Terrier project by ScottyLabs.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

For more information about the Terrier project, see the [main README](../README.md).
