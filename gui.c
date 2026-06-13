
#include "gui.h"
#include "font.h"
#include "efi_helpers.h"
#include <efi.h>
#include <efilib.h>

extern EFI_BOOT_SERVICES *BS;
extern EFI_SYSTEM_TABLE *ST;

icon_t* png_load(UINT8 *data, UINTN size);

static const font_t *g_font = &jetbrains_font;

void gui_set_font(const char *name) {

    (void)name;
    g_font = &jetbrains_font;
}

static UINTN isqrt_(UINTN n) {
    if (n == 0) return 0;
    UINTN x = n, y = (x + 1) / 2;
    while (y < x) { x = y; y = (x + n / x) / 2; }
    return x;
}

static UINT32 color_to_u32(color_t c) {
    return (0xFF << 24) | (c.r << 16) | (c.g << 8) | c.b;
}

static UINT32 blend_color(color_t c1, color_t c2, UINT8 alpha) __attribute__((unused));
static UINT32 blend_color(color_t c1, color_t c2, UINT8 alpha) {
    UINT8 r = (c1.r * (255 - alpha) + c2.r * alpha) / 255;
    UINT8 g = (c1.g * (255 - alpha) + c2.g * alpha) / 255;
    UINT8 b = (c1.b * (255 - alpha) + c2.b * alpha) / 255;
    return (0xFF << 24) | (r << 16) | (g << 8) | b;
}

static UINT32* get_pixel(gui_state_t *state, UINTN x, UINTN y) {
    if (x >= state->screen_width || y >= state->screen_height || !state->backbuffer)
        return NULL;
    return &state->backbuffer[y * state->screen_width + x];
}

void gui_present(gui_state_t *state) {
    if (!state->backbuffer) return;

    EFI_STATUS s = uefi_call_wrapper(state->gop->Blt, 10,
        state->gop,
        (EFI_GRAPHICS_OUTPUT_BLT_PIXEL*)state->backbuffer,
        EfiBltBufferToVideo,
        0, 0, 0, 0,
        state->screen_width, state->screen_height,
        0);
    if (!EFI_ERROR(s)) return;

    UINT8 *fb = (UINT8*)state->gop->Mode->FrameBufferBase;
    for (UINTN y = 0; y < state->screen_height; y++) {
        UINT32 *dst = (UINT32*)(fb + y * state->pixels_per_scanline * sizeof(UINT32));
        UINT32 *src = &state->backbuffer[y * state->screen_width];
        for (UINTN x = 0; x < state->screen_width; x++) dst[x] = src[x];
    }
}

EFI_STATUS gui_init(gui_state_t *state) {

    EFI_STATUS status = BS->HandleProtocol(
        ST->ConsoleOutHandle,
        &gEfiGraphicsOutputProtocolGuid,
        (void**)&state->gop
    );

    if (EFI_ERROR(status)) {

        UINTN count;
        EFI_HANDLE *handles = efi_locate_handle_buffer(&gEfiGraphicsOutputProtocolGuid, &count);
        if (!handles) {
            return EFI_NOT_FOUND;
        }
        for (UINTN i = 0; i < count; i++) {
            status = BS->HandleProtocol(handles[i], &gEfiGraphicsOutputProtocolGuid, (void**)&state->gop);
            if (!EFI_ERROR(status)) break;
        }
        efi_free_pool(handles);
    }

    if (EFI_ERROR(status) || !state->gop) {
        return EFI_NOT_FOUND;
    }

    state->screen_width = state->gop->Mode->Info->HorizontalResolution;
    state->screen_height = state->gop->Mode->Info->VerticalResolution;
    state->bpp = 32;
    state->pixel_format = state->gop->Mode->Info->PixelFormat;

    {
        CHAR16 g[160];
        SPrint(g, sizeof(g),
               L"   GOP %dx%d pxfmt=%d ppsl=%d fb=%lx",
               (int)state->screen_width, (int)state->screen_height,
               (int)state->pixel_format,
               (int)state->gop->Mode->Info->PixelsPerScanLine,
               (UINT64)state->gop->Mode->FrameBufferBase);
        efi_log(g);
    }

    state->pixels_per_scanline = state->gop->Mode->Info->PixelsPerScanLine;
    if (state->pixels_per_scanline < state->screen_width) {
        state->pixels_per_scanline = state->screen_width;
    }

    state->bg_color = COLOR_BLACK;
    state->fg_color = COLOR_WHITE;
    state->highlight_color = COLOR_BLUE;

    state->selected = 0;
    state->entries = NULL;
    state->entry_count = 0;
    state->timeout = 0;
    state->timeout_active = 1;
    state->running = 1;
    state->action = VISOR_ACTION_BOOT;
    state->title = NULL;
    state->show_title = 1;
    state->title_color = COLOR_WHITE;
    state->name_color = COLOR_WHITE;
    state->title_size = 0;
    state->name_size = 0;

    state->icon_size = 0;
    state->icon_spacing = 0;
    state->icon_y = 0;

    state->underline_color = COLOR_BLUE;
    state->underline_thickness = 0;
    state->underline_length = 0;

    state->power_position = POWER_POS_BOTTOMRIGHT;
    state->shutdown_color = COLOR_BLUE;
    state->reboot_color = COLOR_BLUE;
    state->firmware_color = COLOR_BLUE;

    state->background = NULL;
    state->background_path = NULL;

    state->backbuffer = efi_allocate_pool(
        state->screen_width * state->screen_height * sizeof(UINT32));
    if (!state->backbuffer) return EFI_OUT_OF_RESOURCES;

    gui_fill_rect(state, 0, 0, state->screen_width, state->screen_height, state->bg_color);
    gui_present(state);

    return EFI_SUCCESS;
}

void gui_fill_rect(gui_state_t *state, UINTN x, UINTN y, UINTN w, UINTN h, color_t color) {
    UINT32 pixel = color_to_u32(color);
    for (UINTN j = y; j < y + h && j < state->screen_height; j++) {
        for (UINTN i = x; i < x + w && i < state->screen_width; i++) {
            UINT32 *p = get_pixel(state, i, j);
            if (p) *p = pixel;
        }
    }
}

static void fill_rect_alpha(gui_state_t *state, INTN x, INTN y, INTN w, INTN h,
                            color_t color, UINT8 alpha) {
    for (INTN j = y; j < y + h; j++) {
        if (j < 0 || j >= (INTN)state->screen_height) continue;
        for (INTN i = x; i < x + w; i++) {
            if (i < 0 || i >= (INTN)state->screen_width) continue;
            UINT32 *p = get_pixel(state, i, j);
            if (!p) continue;
            UINT8 br = (*p >> 16) & 0xFF, bg = (*p >> 8) & 0xFF, bb = *p & 0xFF;
            UINT8 r = (color.r * alpha + br * (255 - alpha)) / 255;
            UINT8 g = (color.g * alpha + bg * (255 - alpha)) / 255;
            UINT8 b = (color.b * alpha + bb * (255 - alpha)) / 255;
            *p = (0xFFu << 24) | (r << 16) | (g << 8) | b;
        }
    }
}

static void fill_round_rect(gui_state_t *state, INTN x, INTN y, INTN w, INTN h,
                            INTN r, color_t color, UINT8 alpha) {
    if (r * 2 > w) r = w / 2;
    if (r * 2 > h) r = h / 2;
    for (INTN j = 0; j < h; j++) {
        INTN inset = 0;
        if (j < r)              { INTN dy = r - 1 - j; inset = r - (INTN)isqrt_(r*r - dy*dy); }
        else if (j >= h - r)    { INTN dy = j - (h - r); inset = r - (INTN)isqrt_(r*r - dy*dy); }
        fill_rect_alpha(state, x + inset, y + j, w - 2 * inset, 1, color, alpha);
    }
}

static void draw_image_sized(gui_state_t *state, icon_t *icon,
                             UINTN x, UINTN y, UINTN size) {
    if (!icon || !icon->pixels || icon->width == 0 || icon->height == 0 || size == 0)
        return;

    for (UINTN j = 0; j < size && (y + j) < state->screen_height; j++) {
        UINTN src_y = j * icon->height / size;
        for (UINTN i = 0; i < size && (x + i) < state->screen_width; i++) {
            UINTN src_x = i * icon->width / size;
            UINT32 pixel = icon->pixels[src_y * icon->width + src_x];
            if (pixel & 0xFF000000) {
                UINT32 *dest = get_pixel(state, x + i, y + j);
                if (dest) *dest = pixel;
            }
        }
    }
}

void gui_draw_image(gui_state_t *state, icon_t *icon, UINTN x, UINTN y) {
    draw_image_sized(state, icon, x, y, ICON_SIZE);
}

static UINTN px_height(UINTN scale) { return scale * 16; }

static UINT8 sample_cov(const unsigned char *cov, UINTN w, UINTN h,
                        UINTN sx256, UINTN sy256) {
    UINTN x0 = sx256 >> 8, y0 = sy256 >> 8;
    if (x0 >= w) x0 = w ? w - 1 : 0;
    if (y0 >= h) y0 = h ? h - 1 : 0;
    UINTN x1 = (x0 + 1 < w) ? x0 + 1 : x0;
    UINTN y1 = (y0 + 1 < h) ? y0 + 1 : y0;
    UINTN fx = sx256 & 0xFF, fy = sy256 & 0xFF;
    UINTN c00 = cov[y0 * w + x0], c10 = cov[y0 * w + x1];
    UINTN c01 = cov[y1 * w + x0], c11 = cov[y1 * w + x1];
    UINTN top = c00 + ((c10 - c00) * fx >> 8);
    UINTN bot = c01 + ((c11 - c01) * fx >> 8);
    return (UINT8)(top + (((INTN)bot - (INTN)top) * (INTN)fy >> 8));
}

static UINTN blend_glyph(gui_state_t *state, const glyph_t *g, UINT32 rgb,
                         INTN dx, INTN dyTop, UINTN size_px, UINTN dh) {
    if (g->w == 0 || g->h == 0) return g->advance * dh / size_px;
    const unsigned char *cov = g_font->pixels + g->pixel_offset;
    UINTN dw = (UINTN)g->w * dh / size_px;
    UINTN ddh = (UINTN)g->h * dh / size_px;
    if (dw == 0 || ddh == 0) return g->advance * dh / size_px;

    UINT8 fr = (rgb >> 16) & 0xFF, fg = (rgb >> 8) & 0xFF, fb = rgb & 0xFF;
    for (UINTN j = 0; j < ddh; j++) {
        UINTN sy = (j * g->h * 256) / ddh;
        INTN py = dyTop + (INTN)j;
        if (py < 0 || py >= (INTN)state->screen_height) continue;
        for (UINTN i = 0; i < dw; i++) {
            UINTN sx = (i * g->w * 256) / dw;
            UINT8 a = sample_cov(cov, g->w, g->h, sx, sy);
            if (!a) continue;
            INTN px = dx + (INTN)i;
            if (px < 0 || px >= (INTN)state->screen_width) continue;
            UINT32 *p = get_pixel(state, px, py);
            if (!p) continue;
            UINT8 br = (*p >> 16) & 0xFF, bg = (*p >> 8) & 0xFF, bb = *p & 0xFF;
            UINT8 r = (fr * a + br * (255 - a)) / 255;
            UINT8 gg = (fg * a + bg * (255 - a)) / 255;
            UINT8 b = (fb * a + bb * (255 - a)) / 255;
            *p = (0xFFu << 24) | (r << 16) | (gg << 8) | b;
        }
    }
    return g->advance * dh / size_px;
}

static UINTN text_width_px(CHAR16 *text, UINTN dh) {
    if (!text) return 0;
    UINTN w = 0;
    while (*text) {
        CHAR16 c = *text++;
        if (c < g_font->first || c > g_font->last) c = '?';
        const glyph_t *g = &g_font->glyphs[c - g_font->first];
        w += (UINTN)g->advance * dh / g_font->size;
    }
    return w;
}

static void draw_text_px(gui_state_t *state, CHAR16 *text, UINTN x, UINTN y,
                         color_t color, UINTN dh) {
    if (!text) return;
    UINT32 rgb = color_to_u32(color) & 0x00FFFFFF;
    UINTN size = g_font->size;
    UINTN baseline = y + (UINTN)g_font->ascent * dh / size;
    INTN pen = (INTN)x;
    while (*text) {
        CHAR16 c = *text++;
        if (c < g_font->first || c > g_font->last) c = '?';
        const glyph_t *g = &g_font->glyphs[c - g_font->first];
        INTN gx = pen + (INTN)((INTN)g->left * (INTN)dh / (INTN)size);
        INTN gyTop = (INTN)baseline - (INTN)((INTN)g->top * (INTN)dh / (INTN)size);
        pen += (INTN)blend_glyph(state, g, rgb, gx, gyTop, size, dh);
    }
}

static UINTN text_width(CHAR16 *text, UINTN scale) {
    return text_width_px(text, px_height(scale));
}
static void draw_text_scaled(gui_state_t *state, CHAR16 *text, UINTN x, UINTN y,
                             color_t color, UINTN scale) {
    draw_text_px(state, text, x, y, color, px_height(scale));
}
static void draw_text_centered(gui_state_t *state, CHAR16 *text, UINTN x, UINTN w,
                               UINTN y, color_t color, UINTN scale) {
    UINTN tw = text_width_px(text, px_height(scale));
    UINTN tx = (tw < w) ? x + (w - tw) / 2 : x;
    draw_text_px(state, text, tx, y, color, px_height(scale));
}

void gui_draw_text_small(gui_state_t *state, CHAR16 *text, UINTN x, UINTN y,
                         color_t color) {
    draw_text_px(state, text, x, y, color, 16);
}
void gui_draw_text_large(gui_state_t *state, CHAR16 *text, UINTN x, UINTN y,
                         color_t color) {
    draw_text_px(state, text, x, y, color, 32);
}

icon_t* gui_load_image(CHAR16 *path) {
    efi_log(L"  image: opening file");
    efi_log(path);
    efi_file_buffer_t *buf = efi_load_file(path);
    if (!buf) { efi_log(L"  ERROR: image file not found or unreadable"); return NULL; }
    { CHAR16 d[64]; SPrint(d, sizeof(d), L"  image: read %d bytes", (int)buf->size); efi_log(d); }

    UINT8 *data = (UINT8*)buf->data;

    UINT8 png_sig[8] = {0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
    INTN is_png = 1;
    for (int i = 0; i < 8; i++) {
        if (data[i] != png_sig[i]) {
            is_png = 0;
            break;
        }
    }

    if (is_png) {
        icon_t *icon = png_load(data, buf->size);
        efi_free_pool(buf);
        if (!icon) efi_log(L"  ERROR: PNG decode failed");
        return icon;
    }

    if (data[0] != 'B' || data[1] != 'M') {
        efi_log(L"  ERROR: image is neither PNG nor BMP");
        efi_free_pool(buf);
        return NULL;
    }

    UINT32 width = *(UINT32*)(data + 18);
    UINT32 height = *(UINT32*)(data + 22);
    UINT16 bpp = *(UINT16*)(data + 28);
    UINT32 compression = *(UINT32*)(data + 30);
    UINT32 data_offset = *(UINT32*)(data + 10);

    if (compression != 0 || (bpp != 24 && bpp != 32)) {
        efi_free_pool(buf);
        return NULL;
    }

    icon_t *icon = efi_allocate_pool(sizeof(icon_t));
    icon->width = width;
    icon->height = height;
    icon->pixels = efi_allocate_pool(width * height * sizeof(UINT32));

    UINT8 *src = data + data_offset;
    UINTN row_padding = (4 - (width * (bpp / 8)) % 4) % 4;

    for (UINTN y = 0; y < height; y++) {
        for (UINTN x = 0; x < width; x++) {
            UINT8 *pixel;
            if (bpp == 32) {
                pixel = src + ((height - 1 - y) * (width * 4 + row_padding * height) + x * 4);
            } else {

                pixel = src + ((height - 1 - y) * (width * 3 + row_padding) + x * 3);
            }

            icon->pixels[y * width + x] = (0xFF << 24) | (pixel[2] << 16) | (pixel[1] << 8) | pixel[0];
        }
    }

    efi_free_pool(buf);
    return icon;
}

icon_t* gui_load_icon(CHAR16 *path) {
    return gui_load_image(path);
}

#define DEFAULT_BACKGROUND_PATH L"\\EFI\\visor\\backgrounds\\default.png"

void gui_set_background(gui_state_t *state, CHAR16 *path) {
    if (state->background && state->background->pixels) {
        efi_free_pool(state->background->pixels);
        efi_free_pool(state->background);
    }
    if (state->background_path) {
        efi_free_pool(state->background_path);
    }

    state->background_path = efi_strdup(path);
    state->background = gui_load_image(path);

    if (!state->background && efi_strcmp(path, DEFAULT_BACKGROUND_PATH) != 0) {
        efi_log(L"  WARN: background unusable - falling back to default background");
        state->background = gui_load_image(DEFAULT_BACKGROUND_PATH);
        if (state->background)
            efi_log(L"  background: default fallback loaded");
        else
            efi_log(L"  WARN: default background missing too - using solid colour");
    }
}

void gui_draw_background(gui_state_t *state) {
    if (!state->background || !state->background->pixels) {

        gui_fill_rect(state, 0, 0, state->screen_width, state->screen_height, state->bg_color);
        return;
    }

    icon_t *bg = state->background;
    UINTN dst_width = state->screen_width;
    UINTN dst_height = state->screen_height;

    for (UINTN y = 0; y < dst_height; y++) {
        for (UINTN x = 0; x < dst_width; x++) {
            UINTN src_x = (x * bg->width) / dst_width;
            UINTN src_y = (y * bg->height) / dst_height;
            UINT32 pixel = bg->pixels[src_y * bg->width + src_x];
            UINT32 *dest = get_pixel(state, x, y);
            if (dest) {

                *dest = (pixel & 0x00FFFFFF) | 0xFF000000;
            }
        }
    }
}

static const struct { CHAR16 *label; int action; } POWER_ACTIONS[] = {
    { L"Shutdown", VISOR_ACTION_SHUTDOWN },
    { L"Reboot",   VISOR_ACTION_REBOOT   },
    { L"Firmware", VISOR_ACTION_FIRMWARE },
};
#define POWER_ACTION_COUNT 3

static void draw_power_actions(gui_state_t *state) {
    UINTN scale   = 2;
    UINTN line_h  = 8 * scale + 12;
    UINTN margin  = 30;
    UINTN block_h = POWER_ACTION_COUNT * line_h;

    int right_side = (state->power_position == POWER_POS_BOTTOMRIGHT ||
                      state->power_position == POWER_POS_TOPRIGHT);
    int top_side   = (state->power_position == POWER_POS_TOPRIGHT ||
                      state->power_position == POWER_POS_TOPLEFT);

    UINTN top = top_side ? margin : (state->screen_height - margin - block_h);

    color_t dim = { 0xC0, 0xC0, 0xC8 };
    color_t key_color[POWER_ACTION_COUNT] = {
        state->shutdown_color, state->reboot_color, state->firmware_color
    };

    for (UINTN i = 0; i < POWER_ACTION_COUNT; i++) {
        CHAR16 *label = POWER_ACTIONS[i].label;
        UINTN  tw = text_width(label, scale);
        UINTN  x  = right_side ? (state->screen_width - margin - tw) : margin;
        UINTN  y  = top + i * line_h;

        CHAR16 first[2] = { label[0], 0 };
        draw_text_scaled(state, first, x, y, key_color[i], scale);
        draw_text_scaled(state, label + 1, x + (8 + 2) * scale, y, dim, scale);
    }
}

void gui_draw_menu(gui_state_t *state) {

    gui_draw_background(state);

    fill_rect_alpha(state, 0, 0, state->screen_width, state->screen_height,
                    COLOR_BLACK, 60);

    if (state->show_title) {
        CHAR16 *title = (state->title && state->title[0]) ? state->title : L"Visor";
        UINTN title_px = state->title_size ? state->title_size
                                           : state->screen_height / 12;
        UINTN tw = text_width_px(title, title_px);
        UINTN tx = (tw < state->screen_width) ? (state->screen_width - tw) / 2 : 0;
        draw_text_px(state, title, tx, state->screen_height / 14,
                     state->title_color, title_px);
    }

    if (state->entry_count == 0) {
        CHAR16 msg[] = L"No boot entries found";
        draw_text_centered(state, msg, 0, state->screen_width,
                           state->screen_height / 2, state->fg_color, 2);
        draw_power_actions(state);
        return;
    }

    UINTN is      = state->icon_size    ? state->icon_size    : ICON_SIZE;
    UINTN isp     = state->icon_spacing ? state->icon_spacing : ICON_SPACING + 40;
    UINTN slot    = is + isp;
    UINTN row_w   = state->entry_count * slot - (slot - is);
    UINTN start_x = (state->screen_width > row_w) ? (state->screen_width - row_w) / 2 : 0;
    UINTN icon_cy = state->icon_y ? state->icon_y : state->screen_height / 2;
    UINTN icon_y  = (icon_cy > is / 2) ? icon_cy - is / 2 : 0;

    UINTN name_px = state->name_size ? state->name_size : 16;
    UINTN ul_th   = state->underline_thickness ? state->underline_thickness : 4;
    INTN  pad     = 16;
    UINTN ul_y    = icon_y + is + 10;
    UINTN name_y  = ul_y + ul_th + 8;

    boot_entry_t *entry = state->entries;
    for (UINTN i = 0; i < state->entry_count; i++) {
        UINTN x = start_x + i * slot;

        if (i == state->selected) {
            INTN card_top = (INTN)icon_y - pad;
            INTN card_bot = (INTN)name_y + (INTN)name_px + pad / 2;
            fill_round_rect(state, (INTN)x - pad, card_top,
                            is + 2 * pad, card_bot - card_top,
                            14, COLOR_WHITE, 38);

            UINTN ul_len = state->underline_length ? state->underline_length
                                                   : (is + 2 * pad - 20);
            INTN  ul_x   = (INTN)x + (INTN)is / 2 - (INTN)ul_len / 2;
            UINTN ul_rad = ul_th / 2; if (ul_rad > 2) ul_rad = 2;
            fill_round_rect(state, ul_x, (INTN)ul_y, (INTN)ul_len, (INTN)ul_th,
                            (INTN)ul_rad, state->underline_color, 230);
        }

        if (entry->icon) {
            draw_image_sized(state, entry->icon, x, icon_y, is);
        } else {
            color_t placeholder = entry->type == 0 ? COLOR_GREEN : COLOR_RED;
            fill_round_rect(state, (INTN)x, (INTN)icon_y, is, is,
                            12, placeholder, 255);
        }

        color_t name_col;
        if (entry->has_color) {
            name_col = entry->color;
        } else if (i == state->selected) {
            name_col = state->name_color;
        } else {
            name_col = (color_t){ state->name_color.r * 7 / 10,
                                  state->name_color.g * 7 / 10,
                                  state->name_color.b * 7 / 10 };
        }
        UINTN nw = text_width_px(entry->name, name_px);
        UINTN nx = (INTN)x + (INTN)is / 2 - (INTN)nw / 2;
        draw_text_px(state, entry->name, nx, name_y, name_col, name_px);

        entry = entry->next;
    }

    draw_power_actions(state);

    if (state->timeout_active && state->timeout > 0) {
        UINT64 elapsed = efi_get_tick() - state->timeout_start;
        INTN remaining = state->timeout - (INTN)(elapsed / 1000);
        if (remaining > 0) {
            CHAR16 buf[40];
            SPrint(buf, sizeof(buf), L"Booting in %ds", (int)remaining);
            UINTN cw = text_width(buf, 2);
            UINTN cx = (state->power_position == POWER_POS_BOTTOMLEFT)
                       ? state->screen_width - 30 - cw : 30;
            draw_text_scaled(state, buf, cx, state->screen_height - 30 - 16,
                             state->fg_color, 2);
        }
    }
}

boot_entry_t* gui_run(gui_state_t *state) {
    EFI_STATUS status;
    EFI_INPUT_KEY key;

    state->timeout_start = efi_get_tick();
    state->action = VISOR_ACTION_BOOT;

    if (state->timeout == 0) {
        state->running = 0;
    }

    INTN last_remaining = -2;
    int  need_redraw = 1;

    while (state->running) {
        if (need_redraw) {
            gui_draw_menu(state);
            gui_present(state);
            need_redraw = 0;
        }

        status = ST->ConIn->ReadKeyStroke(ST->ConIn, &key);
        if (!EFI_ERROR(status)) {

            if (state->timeout_active) { state->timeout_active = 0; need_redraw = 1; }

            CHAR16 uc = key.UnicodeChar;
            if (uc >= 'a' && uc <= 'z') uc -= 32;

            if (uc == 'S') { state->action = VISOR_ACTION_SHUTDOWN; state->running = 0; }
            else if (uc == 'R') { state->action = VISOR_ACTION_REBOOT; state->running = 0; }
            else if (uc == 'F') { state->action = VISOR_ACTION_FIRMWARE; state->running = 0; }
            else if (key.UnicodeChar == 0x0D) { state->running = 0; }
            else if (key.UnicodeChar == 0x00) {
                switch (key.ScanCode) {
                    case 0x01:
                    case 0x04:
                        if (state->selected > 0) state->selected--;
                        else state->selected = state->entry_count - 1;
                        need_redraw = 1;
                        break;
                    case 0x02:
                    case 0x03:
                        if (state->selected < state->entry_count - 1) state->selected++;
                        else state->selected = 0;
                        need_redraw = 1;
                        break;
                    case 0x17:
                        state->running = 0;
                        break;
                }
            }
            else if (key.UnicodeChar >= '1' && key.UnicodeChar <= '9') {
                UINTN idx = key.UnicodeChar - '1';
                if (idx < state->entry_count) { state->selected = idx; state->running = 0; }
            }
        }

        if (state->timeout_active && state->timeout > 0) {
            UINT64 elapsed = efi_get_tick() - state->timeout_start;
            INTN remaining = state->timeout - (INTN)(elapsed / 1000);
            if (remaining <= 0) {
                state->running = 0;
            } else if (remaining != last_remaining) {
                last_remaining = remaining;
                need_redraw = 1;
            }
        }

        efi_sleep(30);
    }

    if (state->action != VISOR_ACTION_BOOT) return NULL;

    boot_entry_t *selected = state->entries;
    for (UINTN i = 0; i < state->selected && selected; i++) {
        selected = selected->next;
    }
    return selected;
}

void gui_shutdown(gui_state_t *state) {

    boot_entry_t *entry = state->entries;
    while (entry) {
        if (entry->icon && entry->icon->pixels) {
            efi_free_pool(entry->icon->pixels);
            efi_free_pool(entry->icon);
        }
        entry = entry->next;
    }

    if (state->background && state->background->pixels) {
        efi_free_pool(state->background->pixels);
        efi_free_pool(state->background);
    }
    if (state->background_path) {
        efi_free_pool(state->background_path);
    }

    gui_fill_rect(state, 0, 0, state->screen_width, state->screen_height, COLOR_BLACK);
    gui_present(state);

    if (state->backbuffer) {
        efi_free_pool(state->backbuffer);
        state->backbuffer = NULL;
    }
}
