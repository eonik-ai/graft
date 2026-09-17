# SPDX-License-Identifier: Apache-2.0
# OpenWrt packager recipe. Submit via https://github.com/openwrt/packages
# Needs the rust host build (lang/rust). ffmpeg is a runtime package.
include $(TOPDIR)/rules.mk

PKG_NAME:=graft
PKG_VERSION:=0.2.2
PKG_RELEASE:=1

PKG_SOURCE_PROTO:=git
PKG_SOURCE_URL:=https://github.com/eonik-ai/graft.git
PKG_SOURCE_VERSION:=v$(PKG_VERSION)
PKG_MIRROR_HASH:=skip

PKG_MAINTAINER:=eonik <connect@eonik.ai>
PKG_LICENSE:=Apache-2.0
PKG_LICENSE_FILES:=LICENSE

PKG_BUILD_DEPENDS:=rust/host
PKG_BUILD_PARALLEL:=1

include $(INCLUDE_DIR)/package.mk

define Package/graft
  SECTION:=multimedia
  CATEGORY:=Multimedia
  TITLE:=Change the hook. Keep the body.
  URL:=https://github.com/eonik-ai/graft
  DEPENDS:=+ffmpeg
endef

define Package/graft/description
  Local-first composition workspace. Incremental compiler for video.
  Git owns recipe history; graft owns scions, layers, and compile.
endef

define Build/Compile
	cd $(PKG_BUILD_DIR) && \
		cargo build --release --locked --bin graft
endef

define Package/graft/install
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/target/release/graft $(1)/usr/bin/
endef

$(eval $(call BuildPackage,graft))
