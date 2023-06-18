################################################################################
#
# spectrometer_sbc
#
################################################################################

SPECTROMETER_SBC_VERSION = 0.1.0
SPECTROMETER_SBC_LICENSE = MIT
SPECTROMETER_SBC_LICENSE_FILES = COPYING

SPECTROMETER_SBC_SUBDIR = spectrometer_sbc

SPECTROMETER_SBC_SITE = ../..
SPECTROMETER_SBC_SITE_METHOD = local
SPECTROMETER_SBC_OVERRIDE_SRCDIR_RSYNC_EXCLUSIONS = \
	--exclude sbc_config --exclude *.nix

# TODO: Add tailwindcss as a host dependency instead of relying on Nix to provide it
SPECTROMETER_SBC_DEPENDENCIES = udev host-wasm-pack

# Default cargo post-processing expects sources to be archived, using custom scipt to avoid that
RSYNC_POST_PROCESS = '$(SPECTROMETER_SBC_PKGDIR)rsync-post-process'
define SPECTROMETER_SBC_RSYNC_POST_PROCCESS
	$(EXTRA_ENV) $(SPECTROMETER_SBC_DL_ENV) $(RSYNC_POST_PROCESS) -b '$(@D)'
endef
SPECTROMETER_SBC_POST_RSYNC_HOOKS += SPECTROMETER_SBC_RSYNC_POST_PROCCESS

define SPECTROMETER_SBC_INSTALL_INIT_SYSV
	$(INSTALL) -D -m 0755 $(SPECTROMETER_SBC_PKGDIR)init.sh \
    						$(TARGET_DIR)/etc/init.d/S99spectrometer-sbc
	$(INSTALL) -D -m 0644 $(SPECTROMETER_SBC_PKGDIR)env \
    						$(TARGET_DIR)/etc/default/spectrometer_sbc
endef

SPECTROMETER_SBC_CARGO_ENV += LASER_PIN='$(BR2_PACKAGE_SPECTROMETER_SBC_LASER_PIN)'
SPECTROMETER_SBC_CARGO_ENV += BUTTON_PIN='$(BR2_PACKAGE_SPECTROMETER_SBC_BUTTON_PIN)'

# Randomly generated number, needs to be consistent across ssr and hydrate packages
SPECTROMETER_SBC_CARGO_ENV += SERVER_FN_OVERRIDE_KEY='8306904707'

# Default build command + WASM build step
define SPECTROMETER_SBC_BUILD_CMDS
	cd $(SPECTROMETER_SBC_SRCDIR) && \
	$(TARGET_MAKE_ENV) \
		$(TARGET_CONFIGURE_OPTS) \
		$(PKG_CARGO_ENV) \
		$(SPECTROMETER_SBC_CARGO_ENV) \
		cargo build \
			$(if $(BR2_ENABLE_DEBUG),,--release) \
			--offline --manifest-path Cargo.toml --locked \
			--no-default-features --features=ssr
	cd $(SPECTROMETER_SBC_SRCDIR) && \
	$(TARGET_MAKE_ENV) \
		$(TARGET_CONFIGURE_OPTS) \
		$(PKG_CARGO_ENV) \
		$(SPECTROMETER_SBC_CARGO_ENV) \
		wasm-pack build \
			--out-dir pkg --target web \
			$(if $(BR2_ENABLE_DEBUG),--dev,--release) \
			-- \
			--offline --manifest-path Cargo.toml --locked \
			--no-default-features --features=hydrate
	cd $(SPECTROMETER_SBC_SRCDIR) && \
		tailwindcss --input style/input.css --output style/output.css
endef

# cargo-package uses `cargo install`, which only installs binaries. Copy WASM manually 
define SPECTROMETER_SBC_INSTALL_WASM
	$(Q)mkdir -p $(TARGET_DIR)/usr/share/spectrometer_sbc/pkg
	$(Q)cp -r $(SPECTROMETER_SBC_SRCDIR)/pkg $(TARGET_DIR)/usr/share/spectrometer_sbc/
	find $(TARGET_DIR)/usr/share/spectrometer_sbc/pkg/ -type f -exec sed -Ei 's/([a-zA-Z_]+)_bg.wasm/\1.wasm/p' {} \;
	for bg_file in $(TARGET_DIR)/usr/share/spectrometer_sbc/pkg/*_bg*; do \
		fixed_file="$$(echo $$bg_file | sed -En 's/(.*)_bg(.*)/\1\2/p')" ; \
		mv $$bg_file $$fixed_file ; \
	done
	$(Q)cp $(SPECTROMETER_SBC_SRCDIR)/style/output.css $(TARGET_DIR)/usr/share/spectrometer_sbc/pkg/spectrometer_sbc.css
endef
SPECTROMETER_SBC_POST_BUILD_HOOKS += SPECTROMETER_SBC_INSTALL_WASM

$(eval $(cargo-package))
