/* Original adapter for libretro.h's variadic log callback; stable Rust cannot define it. */
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include "libretro.h"
void nibcase_core_log(enum retro_log_level level, const char *format, ...) {
    if (level == RETRO_LOG_DEBUG && !getenv("NIBCASE_DEBUG")) return;
    char message[4096];
    va_list args;
    va_start(args, format);
    vsnprintf(message, sizeof(message), format, args);
    va_end(args);
    fprintf(stderr, "nibcase core: %s", message);
}
