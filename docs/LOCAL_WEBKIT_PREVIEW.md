# Local WebKit preview

Pasted can run against a locally built WebKit on macOS without changing its ordinary build, release artifacts, or dependency graph. This path is intended for WebKit development and before/after demonstrations; it is not a distributable Pasted build.

Build WebKit, install Pasted's locked dependencies, then pass the WebKit build-products directory to the preview launcher:

```sh
npm ci
npm run dev:local-webkit -- /absolute/path/to/WebKitBuild/Release
```

The launcher builds Pasted's normal debug executable and CLI, starts Vite on loopback, and launches the executable directly with framework and XPC loader paths pointed at the supplied build. It uses `vmmap` to verify that the Pasted process actually loaded that `WebKit.framework`; a successful application launch without that verification is treated as a failure.

For recording safety, the launcher creates and seeds a temporary demonstration database, starts the preview with clipboard capture paused, and deletes that temporary database when the preview exits. It never opens or modifies the ordinary Pasted library. Quit Pasted to stop the preview and its Vite server. Set `PASTED_LOCAL_WEBKIT_SKIP_BUILD=1` to reuse the existing debug executables during repeated recording sessions.

The launcher does not bundle WebKit, modify Pasted's release configuration, replace the system framework, or alter the release build's database selection. The database override is compiled only into debug builds.
