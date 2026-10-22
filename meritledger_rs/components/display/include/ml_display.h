#pragma once
// The Waveshare ESP32-S3-Touch-LCD-1.47's JD9853 panel (320 x 172, used in
// landscape), driven in horizontal strips from Rust.
#include <stdbool.h>
#include <stdint.h>
#include "esp_err.h"

// Initializes the SPI bus, the panel and the backlight. Call once.
esp_err_t ml_display_init(void);
// The DMA-capable strip buffer: 320 x 24 RGB565 pixels, native byte order.
uint16_t *ml_display_buffer(void);
// Landscape one way up, or turned 180 degrees.
esp_err_t ml_display_orient(bool flip);
// Sends `rows` rows of the buffer to screen rows top..top+rows (swapping
// each pixel's bytes first when `swap`); waits for the DMA to finish.
esp_err_t ml_display_flush(int top, int rows, bool swap);
// Backlight 0-100 %. Waveshare recommends at most 50.
esp_err_t ml_display_backlight(int percent);
