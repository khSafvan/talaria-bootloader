
#include "windows_boot.h"
#include "efi_helpers.h"
#include <efi.h>
#include <efilib.h>

extern EFI_BOOT_SERVICES *BS;
extern EFI_SYSTEM_TABLE *ST;
extern EFI_HANDLE IH;

EFI_DEVICE_PATH* windows_make_file_path(EFI_HANDLE handle, CHAR16 *filename) {
    EFI_DEVICE_PATH *dp = NULL;
    BS->HandleProtocol(handle, &gEfiDevicePathProtocolGuid, (void**)&dp);
    if (!dp) return NULL;

    UINTN dp_len    = DevicePathSize(dp) - sizeof(EFI_DEVICE_PATH_PROTOCOL);
    UINTN fname_len = StrLen(filename) * sizeof(CHAR16);
    UINTN fp_size   = sizeof(FILEPATH_DEVICE_PATH) + fname_len;
    UINTN end_size  = sizeof(EFI_DEVICE_PATH_PROTOCOL);
    UINTN total_len = dp_len + fp_size + end_size;

    UINT8 *new_dp = efi_allocate_pool(total_len);
    if (!new_dp) return NULL;

    CopyMem(new_dp, dp, dp_len);

    FILEPATH_DEVICE_PATH *fp = (FILEPATH_DEVICE_PATH*)(new_dp + dp_len);
    fp->Header.Type    = MEDIA_DEVICE_PATH;
    fp->Header.SubType = MEDIA_FILEPATH_DP;
    SetDevicePathNodeLength(&fp->Header, (UINT16)fp_size);
    StrCpy(fp->PathName, filename);

    EFI_DEVICE_PATH_PROTOCOL *end = (EFI_DEVICE_PATH_PROTOCOL*)(new_dp + dp_len + fp_size);
    end->Type    = END_DEVICE_PATH_TYPE;
    end->SubType = END_ENTIRE_DEVICE_PATH_SUBTYPE;
    SetDevicePathNodeLength(end, (UINT16)end_size);

    return (EFI_DEVICE_PATH*)new_dp;
}

static EFI_HANDLE find_esp(void) {
    UINTN count;
    EFI_HANDLE *handles = efi_locate_handle_buffer(&gEfiSimpleFileSystemProtocolGuid, &count);
    if (!handles) return NULL;

    for (UINTN i = 0; i < count; i++) {
        EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *fs;
        EFI_STATUS status = BS->HandleProtocol(handles[i], &gEfiSimpleFileSystemProtocolGuid, (void**)&fs);
        if (EFI_ERROR(status)) continue;

        EFI_FILE_PROTOCOL *root;
        status = fs->OpenVolume(fs, &root);
        if (EFI_ERROR(status)) continue;

        if (efi_file_exists(L"\\EFI")) {
            root->Close(root);
            EFI_HANDLE esp = handles[i];
            efi_free_pool(handles);
            return esp;
        }
        root->Close(root);
    }

    efi_free_pool(handles);
    return NULL;
}

EFI_STATUS windows_find_bootmgr(CHAR16 *partition_uuid, EFI_DEVICE_PATH **bootmgr_path) {
    (void)partition_uuid;

    CHAR16 *paths[] = {
        L"\\EFI\\Microsoft\\Boot\\bootmgfw.efi",
        L"\\EFI\\BOOT\\bootmgfw.efi",
        L"\\EFI\\BOOT\\BOOTX64.EFI",
        NULL
    };

    EFI_HANDLE esp = find_esp();
    if (!esp) return EFI_NOT_FOUND;

    for (int i = 0; paths[i]; i++) {
        efi_file_t *f = efi_fopen(paths[i]);
        if (f) {
            efi_fclose(f);
            *bootmgr_path = windows_make_file_path(esp, paths[i]);
            if (*bootmgr_path) return EFI_SUCCESS;
        }
    }

    return EFI_NOT_FOUND;
}

EFI_STATUS efi_chainload(EFI_HANDLE image_handle,
                          EFI_SYSTEM_TABLE *system_table,
                          CHAR16 *path) {
    (void)system_table;

    efi_log(L"win: chainloading via device path");
    efi_log(path);
    efi_print(L"Chainloading: ");
    efi_print(path);
    efi_print(L"\r\n");

    UINTN count = 0;
    EFI_HANDLE *handles = efi_locate_handle_buffer(&gEfiSimpleFileSystemProtocolGuid, &count);
    if (!handles) {
        efi_log(L"ERROR: no filesystems found to locate the EFI binary");
        efi_print(L"No filesystems\r\n");
        return EFI_NOT_FOUND;
    }

    EFI_DEVICE_PATH *dp = NULL;
    for (UINTN i = 0; i < count; i++) {
        EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *fs;
        if (EFI_ERROR(BS->HandleProtocol(handles[i],
                &gEfiSimpleFileSystemProtocolGuid, (void**)&fs))) continue;
        EFI_FILE_PROTOCOL *root;
        if (EFI_ERROR(fs->OpenVolume(fs, &root))) continue;
        EFI_FILE_PROTOCOL *fh;
        EFI_STATUS s = root->Open(root, &fh, path, EFI_FILE_MODE_READ, 0);
        root->Close(root);
        if (!EFI_ERROR(s)) {
            fh->Close(fh);
            dp = windows_make_file_path(handles[i], path);
            break;
        }
    }
    efi_free_pool(handles);

    if (!dp) {
        efi_log(L"ERROR: EFI binary not found on any volume");
        efi_print(L"Failed to open file\r\n");
        return EFI_NOT_FOUND;
    }

    efi_log(L"win: LoadImage() from device path");
    EFI_HANDLE new_handle;
    EFI_STATUS status = BS->LoadImage(FALSE, image_handle, dp, NULL, 0, &new_handle);
    efi_free_pool(dp);

    if (EFI_ERROR(status)) {
        efi_log(L"ERROR: LoadImage failed for the EFI binary");
        efi_print(L"LoadImage failed\r\n");
        return status;
    }

    efi_log(L"win: StartImage() - handing control to Windows Boot Manager");
    status = BS->StartImage(new_handle, NULL, NULL);
    if (EFI_ERROR(status)) {
        efi_log(L"ERROR: StartImage returned - chainload failed");
        efi_print(L"StartImage failed\r\n");
    }
    return status;
}

EFI_STATUS windows_boot(boot_entry_t *entry, EFI_SYSTEM_TABLE *st) {
    efi_log(L"win: booting Windows");
    efi_print(L"Booting Windows\r\n");

    if (entry->kernel_path) {
        return efi_chainload(IH, st, entry->kernel_path);
    }

    efi_log(L"win: no explicit path, searching for bootmgfw.efi");
    EFI_DEVICE_PATH *bootmgr_path = NULL;
    EFI_STATUS status = windows_find_bootmgr(entry->uuid, &bootmgr_path);
    if (EFI_ERROR(status)) {
        efi_log(L"ERROR: Windows Boot Manager (bootmgfw.efi) not found");
        efi_print(L"Windows Boot Manager not found\r\n");
        return status;
    }

    EFI_HANDLE new_handle;
    status = BS->LoadImage(FALSE, IH, bootmgr_path, NULL, 0, &new_handle);
    if (EFI_ERROR(status)) {
        efi_log(L"ERROR: LoadImage failed for bootmgfw.efi");
        efi_print(L"LoadImage failed for bootmgfw.efi\r\n");
        return status;
    }

    status = BS->StartImage(new_handle, NULL, NULL);
    if (EFI_ERROR(status)) {
        efi_print(L"StartImage failed\r\n");
    }
    return status;
}
