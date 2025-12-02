#include <stdio.h>
#include <sys/socket.h>
#include <unistd.h>

#include <lio.h>

void test_callback(int32_t result) {
    printf("Callback received: %d\nC bindings are is working\n", result);
}

int main(void) {
    lio_close(999, test_callback);
    sleep(1);
    return 0;
}

