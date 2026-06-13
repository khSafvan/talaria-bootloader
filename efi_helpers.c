#include "efi_helpers.h"
#include <efi.h>
#include <efilib.h>
#include <stdarg.h>

extern EFI_BOOT_SERVICES *BS;
extern EFI_SYSTEM_TABLE *ST;

void* efi_allocate_pool(UINTN size) {
    void *ptr = NULL;
    BS->AllocatePool(EfiLoaderData, size, &ptr);
    return ptr;
}

void efi_free_pool(void *ptr) {
    if (ptr) {
        BS->FreePool(ptr);
    }
}

CHAR16* efi_strdup(CHAR16 *src) {
    if (!src) return NULL;
    UINTN len = 0;
    while (src[len]) len++;
    CHAR16 *dst = efi_allocate_pool((len + 1) * sizeof(CHAR16));
    for (UINTN i = 0; i <= len; i++) {
        dst[i] = src[i];
    }
    return dst;
}

int efi_strcmp(CHAR16 *s1, CHAR16 *s2) {
    if (!s1 || !s2) return 1;
    while (*s1 && *s2 && *s1 == *s2) {
        s1++;
        s2++;
    }
    return *s1 - *s2;
}

CHAR16* efi_strchr(CHAR16 *s, CHAR16 c) {
    while (*s && *s != c) s++;
    return (*s == c) ? s : NULL;
}

efi_file_t* efi_fopen(CHAR16 *path) {
    UINTN count = 0;
    EFI_HANDLE *handles = efi_locate_handle_buffer(
        &gEfiSimpleFileSystemProtocolGuid, &count);
    if (!handles) return NULL;

    EFI_FILE_IO_INTERFACE *io = NULL;
    EFI_FILE_PROTOCOL *root = NULL;

    for (UINTN i = 0; i < count; i++) {
        EFI_STATUS s = BS->HandleProtocol(handles[i],
            &gEfiSimpleFileSystemProtocolGuid, (void**)&io);
        if (!EFI_ERROR(s)) {
            s = io->OpenVolume(io, &root);
            if (!EFI_ERROR(s)) break;
        }
        io = NULL;
        root = NULL;
    }
    efi_free_pool(handles);

    if (!root) return NULL;

    efi_file_t *file = efi_allocate_pool(sizeof(efi_file_t));
    if (!file) {
        root->Close(root);
        return NULL;
    }
    file->root = root;

    if (EFI_ERROR(root->Open(root, &file->handle, path, EFI_FILE_MODE_READ, 0))) {
        root->Close(root);
        efi_free_pool(file);
        return NULL;
    }

    return file;
}

void efi_fclose(efi_file_t *file) {
    if (!file) return;
    if (file->handle) {
        file->handle->Close(file->handle);
    }
    if (file->root) {
        file->root->Close(file->root);
    }
    efi_free_pool(file);
}

UINTN efi_fread(efi_file_t *file, void *buf, UINTN size) {
    if (!file || !file->handle || !buf) return 0;
    file->handle->Read(file->handle, &size, buf);
    return size;
}

int efi_file_exists(CHAR16 *path) {
    efi_file_t *f = efi_fopen(path);
    if (f) {
        efi_fclose(f);
        return 1;
    }
    return 0;
}

efi_file_buffer_t* efi_load_file(CHAR16 *path) {
    efi_file_t *file = efi_fopen(path);
    if (!file) return NULL;

    UINT64 size = 0;
    file->handle->SetPosition(file->handle, ~0ULL);
    file->handle->GetPosition(file->handle, &size);
    file->handle->SetPosition(file->handle, 0);

    efi_file_buffer_t *buf = efi_allocate_pool(sizeof(efi_file_buffer_t));
    if (!buf) {
        efi_fclose(file);
        return NULL;
    }

    buf->data = efi_allocate_pool((UINTN)size);
    if (!buf->data) {
        efi_free_pool(buf);
        efi_fclose(file);
        return NULL;
    }

    buf->size = efi_fread(file, buf->data, (UINTN)size);
    efi_fclose(file);

    return buf;
}

int visor_quiet = 0;

void efi_print(CHAR16 *msg, ...) {
    if (visor_quiet) return;
    if (msg)
        ST->ConOut->OutputString(ST->ConOut, msg);
}

#define LOG_PATH     L"\\EFI\\visor\\boot.log"
#define LOG_MARKER    "===== visor boot ====="
#define LOG_MARKER_W L"===== visor boot ====="
#define LOG_KEEP   3

static EFI_FILE_PROTOCOL *log_open_root(void) {
    UINTN count = 0;
    EFI_HANDLE *handles = efi_locate_handle_buffer(
        &gEfiSimpleFileSystemProtocolGuid, &count);
    if (!handles) return NULL;

    EFI_FILE_IO_INTERFACE *io = NULL;
    EFI_FILE_PROTOCOL *root = NULL;
    for (UINTN i = 0; i < count; i++) {
        if (!EFI_ERROR(BS->HandleProtocol(handles[i],
                &gEfiSimpleFileSystemProtocolGuid, (void**)&io))) {
            if (!EFI_ERROR(io->OpenVolume(io, &root))) break;
        }
        io = NULL; root = NULL;
    }
    efi_free_pool(handles);
    return root;
}

void efi_log(CHAR16 *msg) {
    if (!msg) return;

    EFI_FILE_PROTOCOL *root = log_open_root();
    if (!root) return;

    EFI_FILE_PROTOCOL *f = NULL;
    EFI_STATUS s = root->Open(root, &f, LOG_PATH,
        EFI_FILE_MODE_CREATE | EFI_FILE_MODE_READ | EFI_FILE_MODE_WRITE, 0);
    if (EFI_ERROR(s) || !f) { root->Close(root); return; }

    f->SetPosition(f, 0xFFFFFFFFFFFFFFFFULL);

    UINT8 line[256];
    UINTN n = 0;
    for (UINTN i = 0; msg[i] && n < sizeof(line) - 2; i++) {
        CHAR16 c = msg[i];
        line[n++] = (c < 0x80) ? (UINT8)c : '?';
    }
    line[n++] = '\r';
    line[n++] = '\n';

    UINTN wsize = n;
    f->Write(f, &wsize, line);
    f->Flush(f);
    f->Close(f);
    root->Close(root);
}

void efi_log_begin(void) {
    efi_file_buffer_t *buf = efi_load_file(LOG_PATH);
    UINT8  *keep = NULL;
    UINTN   keep_len = 0;

    if (buf && buf->data && buf->size > 0) {
        UINT8 *d = (UINT8*)buf->data;
        UINTN  sz = buf->size;
        const char *m = LOG_MARKER;
        UINTN mlen = 0; while (m[mlen]) mlen++;

        UINTN offs[64]; UINTN nofs = 0;
        for (UINTN i = 0; i + mlen <= sz && nofs < 64; i++) {
            UINTN k = 0;
            while (k < mlen && d[i + k] == (UINT8)m[k]) k++;
            if (k == mlen) { offs[nofs++] = i; i += mlen - 1; }
        }

        if (nofs >= LOG_KEEP) {
            keep = d + offs[nofs - (LOG_KEEP - 1)];
            keep_len = sz - offs[nofs - (LOG_KEEP - 1)];
        } else if (nofs > 0) {
            keep = d + offs[0];
            keep_len = sz - offs[0];
        }
    }

    EFI_FILE_PROTOCOL *root = log_open_root();
    if (root) {

        EFI_FILE_PROTOCOL *f = NULL;
        if (!EFI_ERROR(root->Open(root, &f, LOG_PATH,
                EFI_FILE_MODE_READ | EFI_FILE_MODE_WRITE, 0)) && f) {
            f->Delete(f);
        }
        if (!EFI_ERROR(root->Open(root, &f, LOG_PATH,
                EFI_FILE_MODE_CREATE | EFI_FILE_MODE_READ | EFI_FILE_MODE_WRITE, 0)) && f) {
            if (keep && keep_len > 0) {
                UINTN w = keep_len;
                f->Write(f, &w, keep);
            }
            f->Flush(f);
            f->Close(f);
        }
        root->Close(root);
    }
    if (buf) { if (buf->data) efi_free_pool(buf->data); efi_free_pool(buf); }

    efi_log(LOG_MARKER_W);
}

void efi_sleep(UINTN milliseconds) {
    BS->Stall(milliseconds * 1000);
}

UINT64 efi_get_tick(void) {
    static EFI_EVENT timer = NULL;
    static UINT64 seconds = 0;

    if (!timer) {
        if (EFI_ERROR(BS->CreateEvent(EVT_TIMER, TPL_APPLICATION,
                                      NULL, NULL, &timer))) {
            timer = NULL;
            return 0;
        }

        BS->SetTimer(timer, TimerPeriodic, 10000000ULL);
        seconds = 0;
        return 0;
    }

    while (BS->CheckEvent(timer) == EFI_SUCCESS) {
        seconds++;
    }
    return seconds * 1000;
}

EFI_HANDLE* efi_locate_handle_buffer(EFI_GUID *proto, UINTN *count) {
    EFI_HANDLE *buffer = NULL;
    EFI_STATUS status = BS->LocateHandleBuffer(ByProtocol, proto, NULL, count, &buffer);
    if (EFI_ERROR(status)) {
        *count = 0;
        return NULL;
    }
    return buffer;
}

EFI_HANDLE efi_get_device_handle(EFI_DEVICE_PATH *dp) {
    if (!dp) return NULL;
    EFI_HANDLE handle = NULL;
    EFI_DEVICE_PATH *remaining = dp;
    EFI_STATUS status = BS->LocateDevicePath(
        &gEfiDevicePathProtocolGuid, &remaining, &handle);
    if (EFI_ERROR(status)) return NULL;
    return handle;
}

void efi_exit_boot_services(EFI_HANDLE image_handle, EFI_SYSTEM_TABLE *system_table) {
    UINTN map_key = 0;
    UINTN map_size = 0;
    EFI_MEMORY_DESCRIPTOR *map = NULL;
    UINTN desc_size;
    UINT32 desc_version;

    system_table->BootServices->GetMemoryMap(&map_size, NULL, &map_key, &desc_size, &desc_version);
    map_size += 2 * desc_size;
    map = (EFI_MEMORY_DESCRIPTOR*)efi_allocate_pool(map_size);

    EFI_STATUS status;
    do {
        status = system_table->BootServices->GetMemoryMap(
            &map_size, map, &map_key, &desc_size, &desc_version);
        if (EFI_ERROR(status)) {
            map_size += desc_size;
            efi_free_pool(map);
            map = (EFI_MEMORY_DESCRIPTOR*)efi_allocate_pool(map_size);
        }
    } while (EFI_ERROR(status));

    status = system_table->BootServices->ExitBootServices(image_handle, map_key);
    if (!EFI_ERROR(status)) {
        system_table->BootServices = NULL;
    }

    efi_free_pool(map);
}
