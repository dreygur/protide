cask "protide" do
  version "0.1.0-alpha.7"
  sha256 "e6064dcee21db583b11533de806cbce2ea3966cb9e2a97bd46a6c9954308b161"

  url "https://github.com/dreygur/protide/releases/download/v#{version}/protide-v#{version}-macos-arm64.dmg"
  name "Protide"
  desc "Native API client for HTTP, GraphQL, WebSocket, gRPC, tRPC and Socket.IO"
  homepage "https://github.com/dreygur/protide"

  depends_on arch: :arm64
  depends_on macos: :monterey

  app "Protide.app"
  binary "#{appdir}/Protide.app/Contents/MacOS/protide"

  # The app is signed ad hoc, not with a Developer ID, so Gatekeeper would
  # refuse to open a downloaded copy. Clearing the quarantine flag lets it run.
  postflight_steps do
    run "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "{{appdir}}/Protide.app"]
  end

  zap trash: "~/Library/Application Support/protide"
end
