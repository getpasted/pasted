#include <stdlib.h>
#include <string.h>

char *pasted_apple_intelligence_request(const char *request) {
  (void)request;
  return strdup("{\"ok\":false,\"code\":\"unsupported_os\",\"message\":\"Apple Intelligence is unavailable in this macOS SDK\"}");
}

void pasted_apple_intelligence_free(char *response) { free(response); }
