#include "termutils.h"

int main() {
  init();
  cursset(0);
  getch(-1);
  return 0;
}
