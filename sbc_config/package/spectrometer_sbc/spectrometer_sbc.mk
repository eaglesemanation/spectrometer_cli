################################################################################
#
# spectrometer_sbc
#
################################################################################

SPECTROMETER_SBC_VERSION = 0.1.0
SPECTROMETER_SBC_SITE = ../..
SPECTROMETER_SBC_SUBDIR = spectrometer_sbc
SPECTROMETER_SBC_OVERRIDE_SRCDIR_RSYNC_EXCLUSIONS = \
	--exclude sbc_config --exclude *.nix
SPECTROMETER_SBC_SITE_METHOD = local
SPECTROMETER_SBC_LICENSE = MIT
SPECTROMETER_SBC_LICENSE_FILES = COPYING

$(eval $(cargo-package))
