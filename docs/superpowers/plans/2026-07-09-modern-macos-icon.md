# Modern macOS App Icon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace KeepAwake’s current blue-and-teal mug icon with the approved premium graphite-and-amber “ember wake” icon and ensure macOS displays the new bundled asset.

**Architecture:** Replace the editable SVG source, render one reviewed 1024 × 1024 master PNG with `rsvg-convert`, derive the standard macOS iconset from that master with `sips`, and compile it into the existing `AppIcon.icns` bundle resource with `iconutil`. Preserve the current Makefile bundling contract, then verify both asset hashes and the icon Finder resolves from the installed application.

**Tech Stack:** SVG, `rsvg-convert`, PNG, macOS `sips`, `iconutil`, Make, shell verification

## Global Constraints

- Use a graphite macOS squircle, bold ivory cup silhouette, and one vivid amber spark-shaped steam stroke.
- Do not reuse the current blue-to-teal gradient or three white steam lines.
- Use no lettering, fine texture, or small details that disappear at Finder and Dock sizes.
- Support every icon size from 16 px through 1024 px.

---

### Task 1: Create and review the master icon

**Files:**
- Modify: `assets/app-icon.svg`
- Create: `assets/app-icon.png`

**Interfaces:**
- Consumes: the approved “ember wake” visual direction
- Produces: an editable vector source and a square 1024 × 1024 RGBA PNG used for iconset rasterization

- [ ] **Step 1: Replace the source artwork**

Replace `assets/app-icon.svg` with a vector-native icon matching this brief:

```text
Use case: logo-brand
Asset type: macOS application icon master, 1024 by 1024 pixels
Primary request: Create a premium icon for an app named KeepAwake. A bold sculpted ivory coffee cup sits inside a graphite macOS squircle. One vivid amber steam stroke rises from the cup and becomes a sharp, elegant spark, symbolizing alertness and energy.
Style/medium: refined dimensional 3D icon, Apple-platform polish, crisp graphic silhouette, restrained depth
Composition/framing: centered, generous optical padding, cup slightly below center, one large steam-spark gesture, readable at 16 pixels
Lighting/mood: dark premium graphite with a warm amber rim light; confident, energetic, sophisticated
Color palette: near-black graphite, warm ivory, saturated amber and restrained orange highlights
Materials/textures: softly sculpted ceramic and subtly satin graphite; clean surfaces
Constraints: exactly one cup and exactly one steam-spark; no words, letters, numbers, borders, watermark, or photographic background; transparent pixels outside the squircle; symmetrical overall balance
Avoid: blue, teal, cyan, purple gradients; three steam lines; saucer; tiny details; clip-art appearance; generic emoji styling; excessive glow
```

Render the raster master:

```bash
rsvg-convert -w 1024 -h 1024 assets/app-icon.svg -o assets/app-icon.png
```

- [ ] **Step 2: Validate source dimensions and appearance**

Run:

```bash
sips -g pixelWidth -g pixelHeight -g hasAlpha assets/app-icon.png
```

Expected: `pixelWidth: 1024`, `pixelHeight: 1024`, and `hasAlpha: yes`. Inspect the source at 1024 px and a 32 px preview; reject it if the old blue/teal palette, three steam lines, illegible spark, or stray text appears.

- [ ] **Step 3: Commit the reviewed source**

```bash
git add assets/app-icon.svg assets/app-icon.png
git commit -m "design: replace macOS app icon artwork"
```

### Task 2: Rebuild and verify the macOS icon bundle

**Files:**
- Modify: `assets/AppIcon.iconset/icon_16x16.png`
- Modify: `assets/AppIcon.iconset/icon_16x16@2x.png`
- Modify: `assets/AppIcon.iconset/icon_32x32.png`
- Modify: `assets/AppIcon.iconset/icon_32x32@2x.png`
- Modify: `assets/AppIcon.iconset/icon_128x128.png`
- Modify: `assets/AppIcon.iconset/icon_128x128@2x.png`
- Modify: `assets/AppIcon.iconset/icon_256x256.png`
- Modify: `assets/AppIcon.iconset/icon_256x256@2x.png`
- Modify: `assets/AppIcon.iconset/icon_512x512.png`
- Modify: `assets/AppIcon.iconset/icon_512x512@2x.png`
- Modify: `assets/AppIcon.icns`

**Interfaces:**
- Consumes: `assets/app-icon.png` at exactly 1024 × 1024
- Produces: the iconset and `assets/AppIcon.icns` consumed by the existing `Makefile` bundle target

- [ ] **Step 1: Derive all required iconset images**

```bash
sips -z 16 16 assets/app-icon.png --out assets/AppIcon.iconset/icon_16x16.png
sips -z 32 32 assets/app-icon.png --out assets/AppIcon.iconset/icon_16x16@2x.png
sips -z 32 32 assets/app-icon.png --out assets/AppIcon.iconset/icon_32x32.png
sips -z 64 64 assets/app-icon.png --out assets/AppIcon.iconset/icon_32x32@2x.png
sips -z 128 128 assets/app-icon.png --out assets/AppIcon.iconset/icon_128x128.png
sips -z 256 256 assets/app-icon.png --out assets/AppIcon.iconset/icon_128x128@2x.png
sips -z 256 256 assets/app-icon.png --out assets/AppIcon.iconset/icon_256x256.png
sips -z 512 512 assets/app-icon.png --out assets/AppIcon.iconset/icon_256x256@2x.png
sips -z 512 512 assets/app-icon.png --out assets/AppIcon.iconset/icon_512x512.png
cp assets/app-icon.png assets/AppIcon.iconset/icon_512x512@2x.png
iconutil -c icns assets/AppIcon.iconset -o assets/AppIcon.icns
```

Expected: each `sips` command reports a written destination and `iconutil` exits successfully.

- [ ] **Step 2: Validate every size and the compiled container**

```bash
file assets/AppIcon.iconset/*.png assets/AppIcon.icns
iconutil -c iconset assets/AppIcon.icns -o /tmp/keepawake-icon-verification.iconset
shasum -a 256 assets/app-icon.png assets/AppIcon.iconset/icon_512x512@2x.png
```

Expected: the files report the standard 16, 32, 64, 128, 256, 512, and 1024 dimensions; the `.icns` round-trip succeeds; and the two SHA-256 values match.

- [ ] **Step 3: Commit the compiled assets**

```bash
git add assets/AppIcon.iconset assets/AppIcon.icns
git commit -m "build: regenerate macOS icon bundle"
```

### Task 3: Install and prove macOS resolves the replacement

**Files:**
- Verify: `Makefile`
- Verify: `Info.plist`
- Verify: `/Applications/KeepAwake.app/Contents/Resources/AppIcon.icns`

**Interfaces:**
- Consumes: `assets/AppIcon.icns` through the existing `make install` flow
- Produces: an installed application whose packaged icon is identical to the repository asset

- [ ] **Step 1: Build and install the application**

```bash
make install
```

Expected: `Built KeepAwake.app` and `Installed to /Applications/KeepAwake.app`.

- [ ] **Step 2: Verify the installed resource and bundle metadata**

```bash
shasum -a 256 assets/AppIcon.icns KeepAwake.app/Contents/Resources/AppIcon.icns /Applications/KeepAwake.app/Contents/Resources/AppIcon.icns
/usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' /Applications/KeepAwake.app/Contents/Info.plist
```

Expected: all three hashes match and the plist value is `AppIcon`.

- [ ] **Step 3: Refresh icon services and inspect the resolved icon**

```bash
touch /Applications/KeepAwake.app
killall Finder 2>/dev/null || true
qlmanage -r cache
```

Expected: commands exit successfully. Inspect `/Applications/KeepAwake.app` in Finder or the Dock and confirm the graphite-and-amber icon is visible instead of the blue-and-teal icon.

- [ ] **Step 4: Run repository checks**

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
git diff --check HEAD~2
```

Expected: formatting, Clippy, tests, and whitespace checks all pass.

- [ ] **Step 5: Publish the existing Stax branch**

Use the repository’s Stax workflow to push `codex/optimize-memory-lifecycle` and update the existing pull request without creating a second PR.
