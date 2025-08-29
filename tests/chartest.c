#include <stdio.h>
int main() {
  for (int i = 0; i < 256; i++)
    printf("%c - %d\n", (char)i, i);
  return 0;
}
