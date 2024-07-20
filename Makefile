PROJECT_NAME := unlimited-ammo
BINARY_PATH := /usr/local/bin

# Default target
all: build install

# Build unlimited-ammo
build:
	cargo build --release

# Install unlimited-ammo
install: build
	sudo cp target/release/${PROJECT_NAME} ${BINARY_PATH}

# Uninstall the binary
# NOTE: This needs to manually ran by the user if they want to uninstall
#				in the root directory of unlimited-ammo run `make uninstall`.
uninstall:
	sudo rm -f $(BINARY_PATH)/$(PROJECT_NAME)

.PHONY: build install uninstall
