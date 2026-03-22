APP_NAME    := KeepAwake
BUNDLE_ID   := com.cesarferreira.keepawake
VERSION     := 0.1.0
BINARY      := keepawake

APP_DIR     := $(APP_NAME).app
CONTENTS    := $(APP_DIR)/Contents
MACOS_DIR   := $(CONTENTS)/MacOS
RESOURCES   := $(CONTENTS)/Resources
INSTALL_DIR := /Applications

.PHONY: build bundle install uninstall clean run

## Build the release binary
build:
	cargo build --release

## Build + package into KeepAwake.app
bundle: build
	@mkdir -p $(MACOS_DIR) $(RESOURCES)
	@cp target/release/$(BINARY) $(MACOS_DIR)/$(BINARY)
	@sed -e 's/$${BUNDLE_ID}/$(BUNDLE_ID)/' \
	     -e 's/$${APP_NAME}/$(APP_NAME)/' \
	     -e 's/$${VERSION}/$(VERSION)/' \
	     Info.plist > $(CONTENTS)/Info.plist
	@echo "Built $(APP_DIR)"

## Install to /Applications (may need sudo)
install: bundle
	@cp -r $(APP_DIR) $(INSTALL_DIR)/$(APP_DIR)
	@echo "Installed to $(INSTALL_DIR)/$(APP_DIR)"

## Remove from /Applications
uninstall:
	@rm -rf $(INSTALL_DIR)/$(APP_DIR)
	@echo "Removed $(INSTALL_DIR)/$(APP_DIR)"

## Clean build artifacts and app bundle
clean:
	cargo clean
	@rm -rf $(APP_DIR)

## Build, bundle, and launch
run: bundle
	@open $(APP_DIR)
