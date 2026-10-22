// JD9853 strip driver for Merit Ledger, on the Waveshare
// ESP32-S3-Touch-LCD-1.47 (172 x 320 panel, used as 320 x 172 landscape).
//
// The init sequence is from CircuitPython's board port for this board
// (ports/espressif/boards/waveshare_esp32_s3_touch_lcd_1_47/board.c,
// MIT, (c) 2025 natheihei, "converted from Arduino example"), with the
// address window and scan direction set for landscape here instead.
// Pins: SPI clock 38, MOSI 39, CS 21, DC 45, reset 40, backlight 46
// (the CircuitPython port and the factory boot log agree).
// No panel-vendor driver: ESP-IDF's generic SPI panel IO sends the
// commands, and drawing is CASET / RASET / RAMWR.
#include "ml_display.h"
#include "driver/gpio.h"
#include "driver/ledc.h"
#include "driver/spi_master.h"
#include "esp_heap_caps.h"
#include "esp_lcd_panel_io.h"
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"
#include "freertos/task.h"

#define WIDTH 320
#define HEIGHT 172
#define STRIP 8
// The 172 visible lines sit in the middle of the controller's 240.
#define OFFSET 34

#define PIN_SCLK 38
#define PIN_MOSI 39
#define PIN_CS 21
#define PIN_DC 45
#define PIN_RST 40
#define PIN_BL 46

static esp_lcd_panel_io_handle_t io;
static uint16_t *pixels;
static SemaphoreHandle_t done;

static bool flushed(esp_lcd_panel_io_handle_t h, esp_lcd_panel_io_event_data_t *event, void *ctx) {
    BaseType_t wake = pdFALSE;
    xSemaphoreGiveFromISR(done, &wake);
    return wake == pdTRUE;
}

#define TRY(x) do { esp_err_t e = (x); if (e) return e; } while (0)
#define CMD(c, ...) do { const uint8_t p[] = {__VA_ARGS__}; \
    TRY(esp_lcd_panel_io_tx_param(io, (c), p, sizeof(p))); } while (0)
#define CMD0(c) TRY(esp_lcd_panel_io_tx_param(io, (c), NULL, 0))

esp_err_t ml_display_init(void) {
    done = xSemaphoreCreateBinary();
    pixels = heap_caps_calloc(WIDTH * STRIP, sizeof(*pixels), MALLOC_CAP_DMA);
    if (!done || !pixels) return ESP_ERR_NO_MEM;

    spi_bus_config_t bus = {.mosi_io_num = PIN_MOSI, .miso_io_num = -1, .sclk_io_num = PIN_SCLK,
        .quadwp_io_num = -1, .quadhd_io_num = -1, .max_transfer_sz = WIDTH * STRIP * 2};
    TRY(spi_bus_initialize(SPI2_HOST, &bus, SPI_DMA_CH_AUTO));
    esp_lcd_panel_io_spi_config_t cfg = {.cs_gpio_num = PIN_CS, .dc_gpio_num = PIN_DC,
        .spi_mode = 0, .pclk_hz = 40 * 1000 * 1000, .trans_queue_depth = 1,
        .lcd_cmd_bits = 8, .lcd_param_bits = 8, .on_color_trans_done = flushed};
    TRY(esp_lcd_new_panel_io_spi((esp_lcd_spi_bus_handle_t)SPI2_HOST, &cfg, &io));

    // Hardware reset.
    gpio_config_t rst = {.pin_bit_mask = 1ULL << PIN_RST, .mode = GPIO_MODE_OUTPUT};
    TRY(gpio_config(&rst));
    gpio_set_level(PIN_RST, 0);
    vTaskDelay(pdMS_TO_TICKS(20));
    gpio_set_level(PIN_RST, 1);
    vTaskDelay(pdMS_TO_TICKS(120));

    CMD0(0x11);  // SLPOUT
    vTaskDelay(pdMS_TO_TICKS(120));
    CMD(0xDF, 0x98, 0x53);
    CMD(0xB2, 0x23);
    CMD(0xB7, 0x00, 0x47, 0x00, 0x6F);
    CMD(0xBB, 0x1C, 0x1A, 0x55, 0x73, 0x63, 0xF0);
    CMD(0xC0, 0x44, 0xA4);
    CMD(0xC1, 0x16);
    CMD(0xC3, 0x7D, 0x07, 0x14, 0x06, 0xCF, 0x71, 0x72, 0x77);
    CMD(0xC4, 0x00, 0x00, 0xA0, 0x79, 0x0B, 0x0A, 0x16, 0x79, 0x0B, 0x0A, 0x16, 0x82);
    CMD(0xC8, 0x3F, 0x32, 0x29, 0x29, 0x27, 0x2B, 0x27, 0x28, 0x28, 0x26, 0x25, 0x17, 0x12, 0x0D, 0x04, 0x00,
              0x3F, 0x32, 0x29, 0x29, 0x27, 0x2B, 0x27, 0x28, 0x28, 0x26, 0x25, 0x17, 0x12, 0x0D, 0x04, 0x00);
    CMD(0xD0, 0x04, 0x06, 0x6B, 0x0F, 0x00);
    CMD(0xD7, 0x00, 0x30);
    CMD(0xE6, 0x14);
    CMD(0xDE, 0x01);
    CMD(0xB7, 0x03, 0x13, 0xEF, 0x35, 0x35);
    CMD(0xC1, 0x14, 0x15, 0xC0);
    CMD(0xC2, 0x06, 0x3A);
    CMD(0xC4, 0x72, 0x12);
    CMD(0xBE, 0x00);
    CMD(0xDE, 0x02);
    CMD(0xE5, 0x00, 0x02, 0x00);
    CMD(0xE5, 0x01, 0x02, 0x00);
    CMD(0xDE, 0x00);
    CMD(0x35, 0x00);  // TEON
    CMD(0x3A, 0x05);  // COLMOD: 16 bits per pixel
    CMD(0xDE, 0x02);
    CMD(0xE5, 0x00, 0x02, 0x00);
    CMD(0xDE, 0x00);
    TRY(ml_display_orient(false));

    // Black before the backlight comes on.
    for (int top = 0; top < HEIGHT; top += STRIP) {
        int rows = HEIGHT - top < STRIP ? HEIGHT - top : STRIP;
        TRY(ml_display_flush(top, rows, false));
    }
    CMD0(0x29);  // DISPON

    ledc_timer_config_t timer = {.speed_mode = LEDC_LOW_SPEED_MODE, .duty_resolution = LEDC_TIMER_10_BIT,
        .timer_num = LEDC_TIMER_0, .freq_hz = 5000, .clk_cfg = LEDC_AUTO_CLK};
    TRY(ledc_timer_config(&timer));
    ledc_channel_config_t backlight = {.gpio_num = PIN_BL, .speed_mode = LEDC_LOW_SPEED_MODE,
        .channel = LEDC_CHANNEL_0, .timer_sel = LEDC_TIMER_0, .duty = 409};  // 40 %
    TRY(ledc_channel_config(&backlight));
    return ESP_OK;
}

uint16_t *ml_display_buffer(void) { return pixels; }

esp_err_t ml_display_orient(bool flip) {
    if (!io) return ESP_ERR_INVALID_STATE;
    // MADCTL: MV (rows and columns swapped: landscape) with MX, or MY for
    // the other way up. RGB order.
    CMD(0x36, flip ? 0xA0 : 0x60);
    return ESP_OK;
}

esp_err_t ml_display_flush(int top, int rows, bool swap) {
    if (!io || top < 0 || rows < 1 || rows > STRIP || top + rows > HEIGHT) return ESP_ERR_INVALID_ARG;
    if (swap) {
        for (int i = 0; i < WIDTH * rows; i++) pixels[i] = (uint16_t)(pixels[i] << 8 | pixels[i] >> 8);
    }
    int y0 = top + OFFSET, y1 = top + rows - 1 + OFFSET;
    CMD(0x2A, 0, 0, (WIDTH - 1) >> 8, (WIDTH - 1) & 0xFF);  // CASET 0..319
    CMD(0x2B, y0 >> 8, y0 & 0xFF, y1 >> 8, y1 & 0xFF);      // RASET
    TRY(esp_lcd_panel_io_tx_color(io, 0x2C, pixels, WIDTH * rows * 2));  // RAMWR
    if (xSemaphoreTake(done, pdMS_TO_TICKS(2000)) != pdTRUE) return ESP_ERR_TIMEOUT;
    return ESP_OK;
}

esp_err_t ml_display_backlight(int percent) {
    if (percent < 0) percent = 0;
    if (percent > 100) percent = 100;
    TRY(ledc_set_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0, percent * 1023 / 100));
    return ledc_update_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0);
}
