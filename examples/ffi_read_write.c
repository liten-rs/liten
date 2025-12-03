// Example demonstrating lio FFI read/write operations
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <lio.h>

void write_callback(int result, uint8_t *buf, size_t buf_len) {
    if (result < 0) {
        fprintf(stderr, "Write failed with error: %d\n", result);
    } else {
        printf("Wrote %d bytes\n", result);
    }

    // Free the buffer after use
    free(buf);
}

void read_callback(int result, uint8_t *buf, size_t buf_len) {
    if (result < 0) {
        fprintf(stderr, "Read failed with error: %d\n", result);
    } else {
        printf("Read %d bytes: ", result);
        fwrite(buf, 1, result, stdout);
        printf("\n");
    }

    // Free the buffer after use
    free(buf);
}

int main() {
    // Example 1: Write to stdout
    const char *message = "Hello from lio FFI!\n";
    size_t msg_len = strlen(message);

    // Allocate buffer (lio takes ownership and returns it via callback)
    uint8_t *write_buf = malloc(msg_len);
    memcpy(write_buf, message, msg_len);

    printf("Writing to stdout...\n");
    lio_write(STDOUT_FILENO, write_buf, msg_len, -1, write_callback);

    // Example 2: Read from a file
    int fd = open("/etc/hostname", O_RDONLY);
    if (fd < 0) {
        perror("open failed");
        return 1;
    }

    // Allocate read buffer
    size_t read_buf_size = 1024;
    uint8_t *read_buf = malloc(read_buf_size);

    printf("Reading from /etc/hostname...\n");
    lio_read(fd, read_buf, read_buf_size, 0, read_callback);

    // Wait for operations to complete
    sleep(1);

    close(fd);
    lio_exit();

    return 0;
}
