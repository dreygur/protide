cask "protide" do
  version "0.1.0-alpha.6"
  sha256 "92e8ba09c498e66956f67eb19b30457184abd5a36046dd68f27741ccaeb48a03"

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
