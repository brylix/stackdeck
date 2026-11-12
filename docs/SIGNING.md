# Signing and notarization

Unsigned builds work locally but every downloader hits Gatekeeper. Adhoc
signing does not help — distribution needs a Developer ID cert and a
notarization round-trip.

## One-time setup

1. Apple Developer Program membership, with the Developer ID Application
   certificate issued to **BRYLIX LLC**.
   - Xcode → Settings → Accounts → Manage Certificates → **+** →
     **Developer ID Application**, or
   - portal → Certificates → **+** (requires a CSR from Keychain Access).

2. Confirm it is present:

   ```bash
   security find-identity -v -p codesigning | grep "Developer ID Application"
   ```

3. Create an **app-specific password** at appleid.apple.com. Not your account
   password, and not the app password used for other tools.

## Building a signed, notarized DMG

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: BRYLIX LLC (8HQRVB7CXJ)"
export APPLE_TEAM_ID=8HQRVB7CXJ
export APPLE_ID=you@example.com
export APPLE_PASSWORD="abcd-efgh-ijkl-mnop"   # app-specific password

cargo tauri build --bundles app,dmg
xcrun stapler staple target/release/bundle/dmg/*.dmg
xcrun stapler validate target/release/bundle/dmg/*.dmg
```

`bundle_dmg.sh` currently fails on this machine; if it does, build with
`--bundles app` and create the disk image by hand:

```bash
hdiutil create -volname Stackdeck -srcfolder path/to/Stage -ov -format UDZO out.dmg
```

Then notarize the DMG directly:

```bash
xcrun notarytool submit out.dmg --apple-id "$APPLE_ID" \
  --team-id "$APPLE_TEAM_ID" --password "$APPLE_PASSWORD" --wait
xcrun stapler staple out.dmg
```

## Verification

```bash
spctl -a -vvv -t exec /Applications/Stackdeck.app   # → accepted, source=Notarized Developer ID
codesign -dvv --entitlements - /Applications/Stackdeck.app
```

## CI

`ci.yml` already passes `APPLE_CERTIFICATE`, `APPLE_SIGNING_IDENTITY`,
`TAURI_SIGNING_PRIVATE_KEY` and password. Add these repository secrets:

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE` | base64 of the exported `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | password set on that `.p12` |
| `APPLE_SIGNING_IDENTITY` | full identity string |
| `APPLE_ID` | Apple ID email |
| `APPLE_PASSWORD` | app-specific password |
| `APPLE_TEAM_ID` | `8HQRVB7CXJ` |

## Note on child processes

Stackdeck spawns Homebrew binaries (`mariadbd`, `php-fpm`, `httpd`) that it does
not sign. They run as separate processes, so signing them is not required, but
hardened runtime must allow unsigned executable memory — see
`src-tauri/entitlements.plist`.
