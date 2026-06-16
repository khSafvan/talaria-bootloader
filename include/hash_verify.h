#ifndef HASH_VERIFY_H
#define HASH_VERIFY_H

#include <efi.h>
#include "gui.h"

int visor_hash_ok(boot_entry_t *entry, const void *data, UINTN size);

int visor_hash_ok_path(boot_entry_t *entry, CHAR16 *path);

#endif
