PREFIX  ?= /usr/local
BIN      = archie
RELEASE  = target/release/$(BIN)

.PHONY: all build install uninstall clean

all: build

build:
	cargo build --release

install: build
	install -Dm755 $(RELEASE) $(DESTDIR)$(PREFIX)/bin/$(BIN)

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/$(BIN)

clean:
	cargo clean
