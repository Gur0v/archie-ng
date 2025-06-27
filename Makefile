CFLAGS = -Wall -Wextra -O2
LDFLAGS = -lreadline
TARGET = archie-ng
SRC = archie.c

$(TARGET): $(SRC)
	$(CC) $(CFLAGS) -o $(TARGET) $(SRC) $(LDFLAGS)

install: $(TARGET)
	install -m 755 $(TARGET) /usr/bin/

uninstall:
	rm -f /usr/bin/$(TARGET)

clean:
	rm -f $(TARGET)

.PHONY: install uninstall clean
