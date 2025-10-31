#include <stdio.h>
#include <stdlib.h>

int main() {
  char *lines = getenv("LINES");
  char *cols = getenv("COLUMNS");

  if (lines != NULL) {
    printf("LINES: %s\n", lines);
  } else {
    printf("LINES is not set\n");
  }

  if (cols != NULL) {
    printf("COLUMNS: %s\n", cols);
  } else {
    printf("COLUMNS is not set\n");
  }

  return 0;
}
