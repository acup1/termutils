#include <stdio.h>
#include <time.h>

int main() {
  clock_t start = clock();

  while ((double)((clock() - start) * 1000) / CLOCKS_PER_SEC < 5000)
    ;
  return 0;
}
