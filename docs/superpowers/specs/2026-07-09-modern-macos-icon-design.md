# Modern macOS App Icon

## Goal

Replace the existing generic blue-and-teal mug icon with a distinctive, modern macOS app icon that communicates “stay awake” and remains legible from 16 px through 1024 px.

## Visual direction

Use a premium “ember wake” treatment:

- A graphite macOS squircle with restrained dimensional depth.
- A bold ivory cup silhouette centered slightly below the optical midpoint.
- One vivid amber steam stroke shaped like a rising spark.
- Soft warm rim lighting and a subtle shadow, without decorative clutter.
- No lettering, fine texture, or small details that disappear at Finder and Dock sizes.

The icon must not reuse the current blue-to-teal gradient or three white steam lines, so the replacement is immediately distinguishable from the existing asset.

## Asset pipeline

Keep a 1024 × 1024 source asset in `assets/`, derive every required macOS iconset size from it, and compile `assets/AppIcon.icns` with `iconutil`. The existing bundle pipeline continues to copy that file to `KeepAwake.app/Contents/Resources/AppIcon.icns`.

## Verification

- Confirm every iconset member has the required pixel dimensions.
- Confirm `iconutil` compiles the iconset successfully.
- Build and reinstall the app.
- Confirm the installed `AppIcon.icns` hash matches the repository asset.
- Inspect the 1024 px source and a small rendered size for visual integrity.
