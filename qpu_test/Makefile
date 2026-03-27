CC = gcc
CFLAGS = -Wall -Wextra -O2
TARGET = qpu_test

$(TARGET): qpu_test.c mailbox.c mailbox.h qpu_hello.h
	$(CC) $(CFLAGS) -o $(TARGET) qpu_test.c mailbox.c

clean:
	rm -f $(TARGET)

.PHONY: clean
