#include <stdio.h>
#include <unistd.h>

#include <lio.h>

void call(int32_t verynice) {
  printf("yay %d", verynice);
}

int main() {
    printf("Testing lio library via pkg-config\n");

    // Initialize lio runtime
    // struct lio_runtime *runtime = lio_runtime_new();
    lio_close(2, call);
    // if (runtime == NULL) {
    //     fprintf(stderr, "Failed to create lio runtime\n");
    //     return 1;
    // }

  sleep(1);
    // printf("Successfully created lio runtime!\n");

    // Clean up
    // lio_runtime_free(runtime);

    return 0;
}
