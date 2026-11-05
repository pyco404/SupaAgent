/* Synthetic CVE-2026-76844 regression. Never load application data or credentials.
 * Exercise blocking ENOSPC -> gzclearerr -> small gzwrite, then normal gzip I/O.
 */
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <zlib.h>

int main(void) {
    const unsigned size = 256U * 1024U;
    unsigned char *input = malloc(size), output[32];
    if (input == NULL) return 10;
    for (unsigned i = 0; i < size; ++i) input[i] = (unsigned char)(i * 17U);
    gzFile failed = gzopen("/dev/full", "wb0");
    if (failed == NULL || gzbuffer(failed, 8192) != 0) return 11;
    errno = 0;
    int first = gzwrite(failed, input, size), code = 0;
    (void)gzerror(failed, &code);
    if (first != 0 || code != Z_ERRNO || errno != ENOSPC) return 12;
    gzclearerr(failed);
    int recovered = gzwrite(failed, "recovery", 8);
    if (recovered != 8) return 13;
    (void)gzclose(failed); /* /dev/full is still full: close may report ENOSPC. */
    free(input);

    const char *path = "synthetic-roundtrip.gz";
    gzFile normal = gzopen(path, "wb");
    if (normal == NULL || gzwrite(normal, "synthetic roundtrip", 19) != 19 ||
        gzclose(normal) != Z_OK) return 14;
    normal = gzopen(path, "rb");
    if (normal == NULL || gzread(normal, output, sizeof(output)) != 19 ||
        memcmp(output, "synthetic roundtrip", 19) != 0 || gzclose(normal) != Z_OK)
        return 15;
    if (remove(path) != 0) return 16;
    puts("gzwrite recovery and normal gzip roundtrip passed");
    return 0;
}
