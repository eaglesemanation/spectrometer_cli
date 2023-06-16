################################################################################
#
# wasm-pack
#
################################################################################

WASM_PACK_VERSION = 0.11.1
WASM_PACK_SITE = $(call github,rustwasm,wasm-pack,v$(WASM_PACK_VERSION))
WASM_PACK_LICENSE = MIT
WASM_PACK_LICENSE_FILES = LICENSE

$(eval $(host-cargo-package))
