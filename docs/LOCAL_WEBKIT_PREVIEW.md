# Local WebKit preview

Pasted can run against a locally built WebKit on macOS without changing its ordinary build, release artifacts, or dependency graph. This path is intended for WebKit development and before/after demonstrations; it is not a distributable Pasted build.

Build WebKit, install Pasted's locked dependencies, then pass the WebKit build-products directory to the preview launcher:

```sh
npm ci
npm run dev:local-webkit -- /absolute/path/to/WebKitBuild/Release
```

The launcher builds Pasted's normal debug executable, starts Vite on loopback, and launches the executable directly with framework and XPC loader paths pointed at the supplied build. It uses `vmmap` to verify that the Pasted process actually loaded that `WebKit.framework`; a successful application launch without that verification is treated as a failure.

Quit Pasted to stop the preview and its Vite server. Set `PASTED_LOCAL_WEBKIT_SKIP_BUILD=1` to reuse the existing debug executable during repeated recording sessions.

The launcher does not bundle WebKit, modify Pasted's release configuration, or replace the system framework. The preview still opens the ordinary local Pasted library, so use a privacy-safe screen such as Settings for recordings unless the library was prepared specifically for public demonstration.
