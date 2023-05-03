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

RSYNC_POST_PROCESS = '$(SPECTROMETER_SBC_PKGDIR)rsync-post-process'

define SPECTROMETER_SBC_RSYNC_POST_PROCCESS
	$(EXTRA_ENV) $(RSYNC_POST_PROCESS) -b '$(@D)'
endef

SPECTROMETER_SBC_POST_RSYNC_HOOKS += SPECTROMETER_SBC_RSYNC_POST_PROCCESS

$(eval $(cargo-package))
