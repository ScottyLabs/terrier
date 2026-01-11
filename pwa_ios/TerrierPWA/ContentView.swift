import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var webViewState = WebViewState()
    
    var body: some View {
        ZStack {
            // Main PWA WebView
            PWAWebView(state: webViewState)
                .edgesIgnoringSafeArea(.all)
            
            // Loading overlay
            if appState.isLoading {
                LoadingView()
            }
            
            // Error overlay
            if appState.hasError {
                ErrorView(message: appState.errorMessage ?? "An error occurred") {
                    appState.clearError()
                    webViewState.reload()
                }
            }
            
            // Offline indicator
            if appState.isOffline {
                VStack {
                    OfflineBanner()
                    Spacer()
                }
            }
        }
        .onReceive(webViewState.$isLoading) { isLoading in
            appState.isLoading = isLoading
        }
        .onReceive(webViewState.$error) { error in
            if let error = error {
                appState.setError(error)
            }
        }
        .onReceive(appState.$pendingURL) { url in
            // Handle deep link URLs passed from the app
            if let url = url {
                print("[UI] 🔗 Loading pending URL: \(url.absoluteString)")
                webViewState.webView?.load(URLRequest(url: url))
                appState.pendingURL = nil
            }
        }
    }
}

struct LoadingView: View {
    var body: some View {
        ZStack {
            Color.black.opacity(0.3)
                .edgesIgnoringSafeArea(.all)
            
            VStack(spacing: 16) {
                ProgressView()
                    .progressViewStyle(CircularProgressViewStyle(tint: .white))
                    .scaleEffect(1.5)
                
                Text("Loading Terrier...")
                    .foregroundColor(.white)
                    .font(.headline)
            }
            .padding(32)
            .background(Color.black.opacity(0.7))
            .cornerRadius(16)
        }
    }
}

struct ErrorView: View {
    let message: String
    let onRetry: () -> Void
    
    var body: some View {
        ZStack {
            Color.black.opacity(0.5)
                .edgesIgnoringSafeArea(.all)
            
            VStack(spacing: 20) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.system(size: 48))
                    .foregroundColor(.yellow)
                
                Text("Connection Error")
                    .font(.title2)
                    .fontWeight(.bold)
                    .foregroundColor(.white)
                
                Text(message)
                    .font(.body)
                    .foregroundColor(.white.opacity(0.8))
                    .multilineTextAlignment(.center)
                    .padding(.horizontal)
                
                Button(action: onRetry) {
                    HStack {
                        Image(systemName: "arrow.clockwise")
                        Text("Retry")
                    }
                    .padding(.horizontal, 32)
                    .padding(.vertical, 12)
                    .background(Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(8)
                }
            }
            .padding(32)
            .background(Color(UIColor.systemBackground).opacity(0.95))
            .cornerRadius(20)
            .shadow(radius: 20)
            .padding(40)
        }
    }
}

struct OfflineBanner: View {
    var body: some View {
        HStack {
            Image(systemName: "wifi.slash")
            Text("You're offline")
            Spacer()
        }
        .padding(.horizontal)
        .padding(.vertical, 8)
        .background(Color.orange)
        .foregroundColor(.white)
        .font(.footnote.weight(.medium))
    }
}

#Preview {
    ContentView()
        .environmentObject(AppState())
}
